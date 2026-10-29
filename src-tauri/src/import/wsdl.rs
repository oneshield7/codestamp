//! A WSDL imported the way SoapUI starts a project: one section per SOAP binding, one request
//! per operation, each with its sample envelope and sent to the binding's first address.

use super::models::{ImportKind, ImportPlan, PlannedRequest, PlannedSection};
use super::postman::{MAX_REQUESTS, MAX_WARNINGS, clean_name, imported_description};
use crate::auth::models::Auth;
use crate::error::AppError;
use crate::soap::{SoapVersion, WsdlDefinition};
use crate::storage::models::{RequestContent, RequestKind, SoapSettings};
use crate::storage::validate::check_request;

/// `location` is where the WSDL was read from, kept on each request so it can be loaded again.
pub fn plan(definition: WsdlDefinition, location: &str) -> Result<ImportPlan, AppError> {
    let mut warnings = definition.warnings;
    let mut sections = Vec::new();
    let mut requests = Vec::new();
    for binding in definition.interfaces {
        let section = sections.len();
        let version = match binding.version {
            SoapVersion::V11 => "1.1",
            SoapVersion::V12 => "1.2",
        };
        sections.push(PlannedSection::named(clean_name(
            &format!("{} · SOAP {version}", binding.name),
            "SOAP",
        )));
        let endpoint = binding.endpoints.first().cloned().unwrap_or_default();
        for operation in binding.operations {
            if requests.len() >= MAX_REQUESTS {
                warnings.push(format!(
                    "Only the first {MAX_REQUESTS} operations were imported"
                ));
                break;
            }
            let content = RequestContent {
                kind: RequestKind::Soap,
                name: clean_name(&operation.name, "Operation"),
                method: "POST".to_owned(),
                url: endpoint.clone(),
                params: Vec::new(),
                headers: Vec::new(),
                body_mode: "xml".to_owned(),
                body: operation.envelope,
                form: Vec::new(),
                body_file: String::new(),
                pre_request_script: String::new(),
                test_script: String::new(),
                auth: Auth::Inherit,
                mcp_operation: None,
                path_variables: Vec::new(),
                graphql_variables: String::new(),
                grpc: None,
                soap: Some(SoapSettings {
                    wsdl: location.to_owned(),
                    version: binding.version,
                    binding: binding.name.clone(),
                    operation: operation.name.clone(),
                    action: operation.action,
                }),
            };
            match check_request(content) {
                Ok(content) => requests.push(PlannedRequest {
                    section: Some(section),
                    content,
                    examples: Vec::new(),
                    description: imported_description(&operation.documentation),
                }),
                Err(error) => warnings.push(format!("{} was left out: {error}", operation.name)),
            }
        }
    }
    if requests.is_empty() {
        return Err(AppError::invalid(
            "This WSDL has no SOAP operations to import",
        ));
    }
    warnings.truncate(MAX_WARNINGS);
    Ok(ImportPlan {
        kind: ImportKind::Collection,
        source: "WSDL 1.1".to_owned(),
        name: clean_name(&definition.name, "SOAP service"),
        description: String::new(),
        auth: Auth::None,
        sections,
        requests,
        variables: Vec::new(),
        environments: Vec::new(),
        icon: None,
        script_count: 0,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::soap::parse_definition;

    const CALCULATOR: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<wsdl:definitions xmlns:wsdl="http://schemas.xmlsoap.org/wsdl/"
    xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/"
    xmlns:soap12="http://schemas.xmlsoap.org/wsdl/soap12/"
    xmlns:s="http://www.w3.org/2001/XMLSchema"
    xmlns:tns="http://tempuri.org/" targetNamespace="http://tempuri.org/">
  <wsdl:types>
    <s:schema elementFormDefault="qualified" targetNamespace="http://tempuri.org/">
      <s:element name="Add"><s:complexType><s:sequence>
        <s:element minOccurs="1" maxOccurs="1" name="intA" type="s:int"/>
        <s:element minOccurs="1" maxOccurs="1" name="intB" type="s:int"/>
      </s:sequence></s:complexType></s:element>
      <s:element name="AddResponse"><s:complexType><s:sequence>
        <s:element minOccurs="1" maxOccurs="1" name="AddResult" type="s:int"/>
      </s:sequence></s:complexType></s:element>
    </s:schema>
  </wsdl:types>
  <wsdl:message name="AddSoapIn"><wsdl:part name="parameters" element="tns:Add"/></wsdl:message>
  <wsdl:message name="AddSoapOut"><wsdl:part name="parameters" element="tns:AddResponse"/></wsdl:message>
  <wsdl:portType name="CalculatorSoap">
    <wsdl:operation name="Add">
      <wsdl:input message="tns:AddSoapIn"/><wsdl:output message="tns:AddSoapOut"/>
    </wsdl:operation>
  </wsdl:portType>
  <wsdl:binding name="CalculatorSoap" type="tns:CalculatorSoap">
    <soap:binding transport="http://schemas.xmlsoap.org/soap/http"/>
    <wsdl:operation name="Add">
      <soap:operation soapAction="http://tempuri.org/Add" style="document"/>
      <wsdl:input><soap:body use="literal"/></wsdl:input>
      <wsdl:output><soap:body use="literal"/></wsdl:output>
    </wsdl:operation>
  </wsdl:binding>
  <wsdl:binding name="CalculatorSoap12" type="tns:CalculatorSoap">
    <soap12:binding transport="http://schemas.xmlsoap.org/soap/http"/>
    <wsdl:operation name="Add">
      <soap12:operation soapAction="http://tempuri.org/Add" style="document"/>
      <wsdl:input><soap12:body use="literal"/></wsdl:input>
      <wsdl:output><soap12:body use="literal"/></wsdl:output>
    </wsdl:operation>
  </wsdl:binding>
  <wsdl:service name="Calculator">
    <wsdl:port name="CalculatorSoap" binding="tns:CalculatorSoap">
      <soap:address location="http://www.dneonline.com/calculator.asmx"/>
    </wsdl:port>
    <wsdl:port name="CalculatorSoap12" binding="tns:CalculatorSoap12">
      <soap12:address location="http://www.dneonline.com/calculator.asmx"/>
    </wsdl:port>
  </wsdl:service>
</wsdl:definitions>"#;

    #[test]
    fn makes_a_section_per_binding_and_a_request_per_operation() {
        let definition = parse_definition(CALCULATOR, &BTreeMap::new()).expect("calculator");
        let plan = plan(definition, "https://example.com/calculator.asmx?wsdl").expect("plan");
        assert_eq!(plan.name, "Calculator");
        assert_eq!(
            plan.section_names(),
            ["CalculatorSoap · SOAP 1.1", "CalculatorSoap12 · SOAP 1.2"]
        );
        assert_eq!(plan.requests.len(), 2);
        let first = &plan.requests[0];
        assert_eq!(first.section, Some(0));
        assert_eq!(first.content.kind, RequestKind::Soap);
        assert_eq!(
            first.content.url,
            "http://www.dneonline.com/calculator.asmx"
        );
        assert_eq!(first.content.body_mode, "xml");
        assert!(first.content.body.contains("<tem:intA>?</tem:intA>"));
        let soap = first.content.soap.as_ref().expect("soap settings");
        assert_eq!(soap.action, "http://tempuri.org/Add");
        assert_eq!(soap.operation, "Add");
        assert_eq!(soap.wsdl, "https://example.com/calculator.asmx?wsdl");
        assert_eq!(
            plan.requests[1]
                .content
                .soap
                .as_ref()
                .map(|soap| soap.version),
            Some(SoapVersion::V12)
        );
    }
}
