use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};

/// Errors returned to the UI. Each variant maps to a stable `code` the frontend can switch on.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("{0}")]
    InvalidInput(String),
    #[error("The request timed out")]
    Timeout,
    #[error("Could not connect: {0}")]
    Connect(String),
    #[error("The request failed: {0}")]
    Request(String),
    #[error("Could not start the HTTP client")]
    ClientInit,
    #[error("{0} was not found")]
    NotFound(&'static str),
    #[error("Could not read or write saved data")]
    Storage,
    #[error("Could not access the secure credential store")]
    SecretStore,
    #[error("{0}")]
    Script(String),
    #[error("Could not read or write the file")]
    FileAccess,
    #[error("Could not update the window")]
    Window,
}

impl AppError {
    pub fn invalid(message: &str) -> Self {
        Self::InvalidInput(message.to_owned())
    }

    const fn code(&self) -> &'static str {
        match self {
            Self::InvalidInput(_) => "invalid_input",
            Self::Timeout => "timeout",
            Self::Connect(_) => "connect",
            Self::Request(_) => "request",
            Self::ClientInit => "client_init",
            Self::NotFound(_) => "not_found",
            Self::Storage => "storage",
            Self::SecretStore => "secret_store",
            Self::Script(_) => "script",
            Self::FileAccess => "file_access",
            Self::Window => "window",
        }
    }
}

// Database details stay out of the message; the UI only needs to know saving failed.
impl From<rusqlite::Error> for AppError {
    fn from(_: rusqlite::Error) -> Self {
        Self::Storage
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut payload = serializer.serialize_struct("AppError", 2)?;
        payload.serialize_field("code", self.code())?;
        payload.serialize_field("message", &self.to_string())?;
        payload.end()
    }
}
