//! SOAP support: reading a WSDL 1.1 (with its schemas and imports) into interfaces and
//! operations with SoapUI-style sample requests, and the XML-to-JSON conversion test scripts
//! use to read SOAP responses.
pub mod commands;
mod load;
mod model;
mod sample;
mod schema;
mod wsdl;
mod xml;
mod xml_json;

pub use load::load_definition;
pub use model::{SoapVersion, WsdlDefinition, WsdlSource};
#[cfg(test)]
pub use wsdl::parse_definition;
pub use xml_json::xml_to_json;
