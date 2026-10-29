use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::session::Session;
use crate::error::AppError;
use crate::storage::db::new_id;

const MAX_SESSIONS: usize = 10;

/// Open MCP sessions by id.
#[derive(Default)]
pub struct McpManager {
    sessions: Mutex<HashMap<String, Arc<Session>>>,
}

impl McpManager {
    pub fn check_capacity(&self) -> Result<(), AppError> {
        if self.lock().len() >= MAX_SESSIONS {
            return Err(capacity_error());
        }
        Ok(())
    }

    pub fn insert(&self, session: Session) -> Result<String, AppError> {
        let mut sessions = self.lock();
        if sessions.len() >= MAX_SESSIONS {
            return Err(capacity_error());
        }
        let id = new_id();
        sessions.insert(id.clone(), Arc::new(session));
        Ok(id)
    }

    pub fn get(&self, id: &str) -> Result<Arc<Session>, AppError> {
        self.lock()
            .get(id)
            .cloned()
            .ok_or(AppError::NotFound("The MCP session"))
    }

    /// Closing a session that has already ended is not an error.
    pub async fn close(&self, id: &str) {
        let session = self.lock().remove(id);
        if let Some(session) = session {
            session.close().await;
        }
    }

    /// Closes every session, for example after the UI reloaded and forgot their ids.
    pub async fn close_all(&self) {
        let sessions: Vec<_> = self.lock().drain().map(|(_, session)| session).collect();
        for session in sessions {
            session.close().await;
        }
    }

    /// The map stays consistent even if a holder panicked, so a poisoned lock is still usable.
    fn lock(&self) -> MutexGuard<'_, HashMap<String, Arc<Session>>> {
        self.sessions.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

fn capacity_error() -> AppError {
    AppError::invalid("Up to 10 MCP servers can be connected at once")
}
