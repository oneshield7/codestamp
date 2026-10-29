use serde_json::Value;

use super::codestamp;
use super::models::ImportPlan;
use super::openapi;
use super::postman::{self, MAX_IMPORT_BYTES};
use crate::error::AppError;

/// Reads an import file: a collection exported by Codestamp, a Postman collection or environment
/// (JSON), or an OpenAPI 3 / Swagger 2 spec (JSON or YAML). The format is recognised from the
/// content, not the file name.
pub fn plan(content: &str) -> Result<ImportPlan, AppError> {
    if content.len() > MAX_IMPORT_BYTES {
        return Err(AppError::invalid("The file is larger than 20 MB"));
    }
    let text = content.trim_start_matches('\u{feff}').trim_start();
    if text.starts_with('{') || text.starts_with('[') {
        let value: Value = serde_json::from_str(text)
            .map_err(|_| AppError::invalid("The file is not valid JSON"))?;
        return if codestamp::is_bundle(&value) {
            codestamp::plan(&value)
        } else if openapi::is_spec(&value) {
            openapi::plan(&value)
        } else {
            postman::plan_value(value)
        };
    }
    // The YAML reader caps aliases, depth and size, so a hostile file cannot blow up memory.
    let value: Value = serde_saphyr::from_str(text).map_err(|_| {
        AppError::invalid("The file is neither JSON nor YAML that Codestamp can read")
    })?;
    if openapi::is_spec(&value) {
        openapi::plan(&value)
    } else {
        Err(AppError::invalid(
            "YAML files are imported as OpenAPI or Swagger specs; this one is neither",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tells_formats_apart() {
        let collection = plan(r#"{ "info": { "name": "C" }, "item": [] }"#).expect("collection");
        assert!(collection.source.starts_with("Collection"));
        let spec = plan("openapi: 3.1.0\ninfo: { title: Y }\npaths: {}\n").expect("yaml spec");
        assert_eq!(spec.source, "OpenAPI 3.1.0");
        assert!(plan("just: yaml\n").is_err_and(|e| e.to_string().contains("OpenAPI or Swagger")));
        assert!(plan("a: &a [*a, *a]\n").is_err());
    }
}
