use uuid::Uuid;

use std::collections::HashSet;

use super::models::{
    FormField, FormFieldKind, GrpcSettings, KeyValue, PathVariable, RequestContent, RequestKind,
    SoapSettings,
};
use crate::auth::apply::check_auth;
use crate::error::AppError;
use crate::request::path_vars::{MAX_PATH_VARIABLES, variable_name};
use crate::request::rules::{
    MAX_KEY_VALUE_LENGTH, MAX_KEY_VALUE_ROWS, MAX_PATH_LENGTH, MAX_REQUEST_BODY_BYTES,
    MAX_SCRIPT_BYTES, MAX_URL_LENGTH, is_allowed_method,
};

pub const MAX_NAME_CHARS: usize = 120;
/// WSDL binding and operation names (XML names) are short; this is generous.
const MAX_SOAP_NAME: usize = 512;
/// MCP requests keep their transport where HTTP requests keep the method.
pub const MCP_TRANSPORTS: [&str; 2] = ["HTTP", "STDIO"];
const BODY_MODES: [&str; 8] = [
    "none",
    "json",
    "text",
    "xml",
    "html",
    "urlencoded",
    "formdata",
    "binary",
];

pub fn clean_name(raw: &str) -> Result<String, AppError> {
    let trimmed = raw.trim();
    let length = trimmed.chars().count();
    if length == 0 {
        return Err(AppError::invalid("Enter a name"));
    }
    if length > MAX_NAME_CHARS {
        return Err(AppError::invalid("Names can be up to 120 characters"));
    }
    Ok(trimmed.to_owned())
}

pub fn check_id(raw: &str) -> Result<(), AppError> {
    Uuid::parse_str(raw)
        .map(|_| ())
        .map_err(|_| AppError::invalid("Unknown item"))
}

pub fn check_optional_id(raw: Option<&str>) -> Result<(), AppError> {
    raw.map_or(Ok(()), check_id)
}

pub fn check_request(content: RequestContent) -> Result<RequestContent, AppError> {
    let name = clean_name(&content.name)?;
    let mut content = content;
    if content.kind == RequestKind::Mcp {
        if !MCP_TRANSPORTS.contains(&content.method.as_str()) {
            return Err(AppError::invalid("Unsupported MCP transport"));
        }
        let unusable = content.mcp_operation.as_ref().is_some_and(|operation| {
            operation.name.is_empty()
                || operation.name.len() > MAX_URL_LENGTH
                || operation.name.chars().any(char::is_control)
        });
        if unusable {
            return Err(AppError::invalid(
                "The MCP tool, prompt or resource name is not valid",
            ));
        }
    } else {
        if !is_allowed_method(&content.method) {
            return Err(AppError::invalid("Unsupported HTTP method"));
        }
        content.mcp_operation = None;
    }
    if content.kind == RequestKind::Graphql {
        if content.method != "POST" {
            return Err(AppError::invalid("GraphQL requests are sent with POST"));
        }
    } else {
        content.graphql_variables.clear();
    }
    if content.kind == RequestKind::Grpc {
        if content.method != "POST" {
            return Err(AppError::invalid("gRPC requests are stored with POST"));
        }
        if let Some(grpc) = &content.grpc {
            check_grpc(grpc)?;
        }
    } else {
        content.grpc = None;
    }
    if content.kind == RequestKind::Soap {
        if content.method != "POST" {
            return Err(AppError::invalid("SOAP requests are sent with POST"));
        }
        if let Some(soap) = &content.soap {
            check_soap(soap)?;
        }
    } else {
        content.soap = None;
    }
    if content.graphql_variables.len() > MAX_REQUEST_BODY_BYTES {
        return Err(AppError::invalid(
            "The GraphQL variables are larger than 10 MB",
        ));
    }
    if content.url.len() > MAX_URL_LENGTH {
        return Err(AppError::invalid("The URL is too long"));
    }
    check_rows(&content.params)?;
    check_rows(&content.headers)?;
    check_path_variables(&content.path_variables)?;
    if !BODY_MODES.contains(&content.body_mode.as_str()) {
        return Err(AppError::invalid("Unsupported body type"));
    }
    if content.body.len() > MAX_REQUEST_BODY_BYTES {
        return Err(AppError::invalid("The request body is larger than 10 MB"));
    }
    check_form(&content.form)?;
    if content.body_file.len() > MAX_PATH_LENGTH {
        return Err(AppError::invalid("The file path is too long"));
    }
    if content.pre_request_script.len() > MAX_SCRIPT_BYTES
        || content.test_script.len() > MAX_SCRIPT_BYTES
    {
        return Err(AppError::invalid("Scripts can be up to 100 KB"));
    }
    check_auth(&content.auth, true)?;
    Ok(RequestContent { name, ..content })
}

fn check_grpc(grpc: &GrpcSettings) -> Result<(), AppError> {
    if grpc.method.len() > MAX_URL_LENGTH || grpc.method.chars().any(char::is_control) {
        return Err(AppError::invalid("The gRPC method name is not valid"));
    }
    if grpc.proto_file.len() > MAX_PATH_LENGTH {
        return Err(AppError::invalid("The file path is too long"));
    }
    Ok(())
}

/// The action ends up in a header, so line breaks and other control characters are refused.
fn check_soap(soap: &SoapSettings) -> Result<(), AppError> {
    let bad = |text: &str, max: usize| text.len() > max || text.chars().any(char::is_control);
    if bad(&soap.wsdl, MAX_PATH_LENGTH.max(MAX_URL_LENGTH)) {
        return Err(AppError::invalid("The WSDL location is not valid"));
    }
    if bad(&soap.binding, MAX_SOAP_NAME) || bad(&soap.operation, MAX_SOAP_NAME) {
        return Err(AppError::invalid("The SOAP operation name is not valid"));
    }
    if bad(&soap.action, MAX_URL_LENGTH) || soap.action.contains('"') {
        return Err(AppError::invalid("The SOAP action is not valid"));
    }
    Ok(())
}

fn check_form(fields: &[FormField]) -> Result<(), AppError> {
    if fields.len() > MAX_KEY_VALUE_ROWS {
        return Err(AppError::invalid("Too many form fields"));
    }
    let too_long = fields.iter().any(|field| {
        let limit = match field.kind {
            FormFieldKind::Text => MAX_KEY_VALUE_LENGTH,
            FormFieldKind::File => MAX_PATH_LENGTH,
        };
        field.key.len() > MAX_KEY_VALUE_LENGTH || field.value.len() > limit
    });
    if too_long {
        return Err(AppError::invalid("A form field is too long"));
    }
    Ok(())
}

fn check_path_variables(variables: &[PathVariable]) -> Result<(), AppError> {
    if variables.len() > MAX_PATH_VARIABLES {
        return Err(AppError::invalid("Too many path variables"));
    }
    let mut seen = HashSet::with_capacity(variables.len());
    for variable in variables {
        if variable_name(&format!(":{}", variable.key)).is_none() || !seen.insert(&variable.key) {
            return Err(AppError::invalid("A path variable name is not valid"));
        }
        if variable.value.len() > MAX_KEY_VALUE_LENGTH {
            return Err(AppError::invalid("A path variable value is too long"));
        }
    }
    Ok(())
}

fn check_rows(rows: &[KeyValue]) -> Result<(), AppError> {
    if rows.len() > MAX_KEY_VALUE_ROWS {
        return Err(AppError::invalid("Too many rows"));
    }
    let too_long = rows
        .iter()
        .any(|row| row.key.len() > MAX_KEY_VALUE_LENGTH || row.value.len() > MAX_KEY_VALUE_LENGTH);
    if too_long {
        return Err(AppError::invalid("A key or value is too long"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::models::RequestKind;

    fn content() -> RequestContent {
        RequestContent {
            kind: RequestKind::Http,
            name: "  List users  ".to_owned(),
            method: "GET".to_owned(),
            url: "{{baseUrl}}/v1/users".to_owned(),
            params: Vec::new(),
            headers: Vec::new(),
            body_mode: "none".to_owned(),
            body: String::new(),
            form: Vec::new(),
            body_file: String::new(),
            pre_request_script: String::new(),
            test_script: String::new(),
            auth: crate::auth::models::Auth::Inherit,
            mcp_operation: None,
            path_variables: Vec::new(),
            graphql_variables: String::new(),
            grpc: None,
            soap: None,
        }
    }

    #[test]
    fn checks_mcp_transports_and_drops_operations_elsewhere() {
        use crate::storage::models::{McpOperation, McpOperationKind};
        let operation = Some(McpOperation {
            kind: McpOperationKind::Prompt,
            name: "summarize".to_owned(),
        });
        let mcp = RequestContent {
            kind: RequestKind::Mcp,
            method: "HTTP".to_owned(),
            mcp_operation: operation.clone(),
            ..content()
        };
        assert!(check_request(mcp.clone()).is_ok_and(|c| c.mcp_operation == operation));
        assert!(
            check_request(RequestContent {
                method: "GET".to_owned(),
                ..mcp.clone()
            })
            .is_err()
        );
        let http = RequestContent {
            mcp_operation: operation,
            ..content()
        };
        assert!(check_request(http).is_ok_and(|c| c.mcp_operation.is_none()));
    }

    #[test]
    fn keeps_grpc_settings_only_on_grpc_requests() {
        let settings = Some(GrpcSettings {
            method: "shop.Shop/GetOrder".to_owned(),
            ..GrpcSettings::default()
        });
        let grpc = RequestContent {
            kind: RequestKind::Grpc,
            method: "POST".to_owned(),
            grpc: settings.clone(),
            ..content()
        };
        assert!(check_request(grpc.clone()).is_ok_and(|c| c.grpc == settings));
        let wrong_method = RequestContent {
            method: "GET".to_owned(),
            ..grpc.clone()
        };
        assert!(check_request(wrong_method).is_err());
        let control = RequestContent {
            grpc: Some(GrpcSettings {
                method: "a\nb".to_owned(),
                ..GrpcSettings::default()
            }),
            ..grpc
        };
        assert!(check_request(control).is_err());
        let http = RequestContent {
            grpc: settings,
            ..content()
        };
        assert!(check_request(http).is_ok_and(|c| c.grpc.is_none()));
    }

    #[test]
    fn trims_names_and_rejects_empty_or_long_ones() {
        assert_eq!(
            clean_name("  Users API ").ok().as_deref(),
            Some("Users API")
        );
        assert!(clean_name("   ").is_err());
        assert!(clean_name(&"x".repeat(MAX_NAME_CHARS + 1)).is_err());
    }

    #[test]
    fn accepts_only_uuid_ids() {
        assert!(check_id("8f2c1a7e-0000-4000-8000-000000000000").is_ok());
        assert!(check_id("1 OR 1=1").is_err());
    }

    #[test]
    fn keeps_urls_with_variables_and_trims_the_name() {
        let checked = check_request(content());
        assert!(checked.is_ok_and(|c| c.name == "List users" && c.url == "{{baseUrl}}/v1/users"));
    }

    #[test]
    fn rejects_unknown_methods_and_body_modes() {
        let mut bad_method = content();
        bad_method.method = "TRACE".to_owned();
        assert!(check_request(bad_method).is_err());

        let mut bad_mode = content();
        bad_mode.body_mode = "yaml".to_owned();
        assert!(check_request(bad_mode).is_err());
    }

    #[test]
    fn rejects_too_many_rows() {
        let mut many = content();
        let row = KeyValue {
            enabled: true,
            key: "k".to_owned(),
            value: "v".to_owned(),
        };
        many.headers = vec![row; MAX_KEY_VALUE_ROWS + 1];
        assert!(check_request(many).is_err());
    }
}
