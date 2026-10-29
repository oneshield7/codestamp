use serde::{Deserialize, Serialize};

/// Warnings kept per definition, so a broken WSDL cannot flood the UI.
const MAX_WARNINGS: usize = 50;

/// Where a WSDL comes from. `Content` is a file dropped on the Import page, so no folder is
/// known for its relative imports.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum WsdlSource {
    Url { url: String },
    File { path: String },
    Content { content: String },
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SoapVersion {
    #[default]
    #[serde(rename = "1.1")]
    V11,
    #[serde(rename = "1.2")]
    V12,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WsdlDefinition {
    /// The first service's name, else the definitions' name, else "SOAP service".
    pub name: String,
    pub target_namespace: String,
    pub interfaces: Vec<SoapInterface>,
    /// Things that could not be read (an import, an undefined type); samples show `?` there.
    pub warnings: Vec<String>,
}

/// One SOAP binding, which SoapUI calls an interface.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoapInterface {
    pub name: String,
    pub version: SoapVersion,
    /// The addresses of the ports that use this binding.
    pub endpoints: Vec<String>,
    pub operations: Vec<SoapOperation>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SoapOperation {
    pub name: String,
    /// The SOAPAction, empty when the binding gives none.
    pub action: String,
    pub documentation: String,
    /// A sample request envelope.
    pub envelope: String,
}

/// Warnings in the order they were found, without repeats.
#[derive(Debug, Default)]
pub(super) struct Warnings(Vec<String>);

impl Warnings {
    pub fn add(&mut self, message: String) {
        if self.0.len() < MAX_WARNINGS && !self.0.contains(&message) {
            self.0.push(message);
        }
    }

    pub fn into_vec(self) -> Vec<String> {
        self.0
    }
}
