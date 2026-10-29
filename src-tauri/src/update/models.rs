use serde::Serialize;

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum UpdateCheck {
    /// This build has no update server or public key, so it never checks.
    #[serde(rename_all = "camelCase")]
    NotConfigured { current: String },
    #[serde(rename_all = "camelCase")]
    UpToDate { current: String },
    #[serde(rename_all = "camelCase")]
    Available {
        current: String,
        version: String,
        notes: Option<String>,
        date: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DownloadProgress {
    pub received: u64,
    pub total: Option<u64>,
}
