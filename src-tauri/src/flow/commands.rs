use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use tauri::State;
use tauri::ipc::Channel;

use super::models::{FlowDetail, FlowInput, FlowSummary};
use super::repo;
use super::run::{self, Context, FlowEvent, FlowRunSummary};
use super::validate::check_flow;
use crate::environment::vault::Vault;
use crate::error::AppError;
use crate::request::client::HttpClient;
use crate::storage::db::Database;
use crate::storage::validate::{check_id, clean_name};

/// Runs in progress, by run id, so Stop can reach them.
#[derive(Default)]
pub struct FlowRuns {
    running: Mutex<HashMap<String, Arc<AtomicBool>>>,
}

impl FlowRuns {
    fn lock(&self) -> MutexGuard<'_, HashMap<String, Arc<AtomicBool>>> {
        self.running.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

#[tauri::command]
pub async fn list_flows(db: State<'_, Database>) -> Result<Vec<FlowSummary>, AppError> {
    db.with(repo::list)
}

#[tauri::command]
pub async fn create_flow(db: State<'_, Database>, name: String) -> Result<FlowSummary, AppError> {
    let name = clean_name(&name)?;
    db.with(|conn| repo::insert(conn, &name))
}

#[tauri::command]
pub async fn get_flow(db: State<'_, Database>, id: String) -> Result<FlowDetail, AppError> {
    check_id(&id)?;
    db.with(|conn| repo::detail(conn, &id))
}

#[tauri::command]
pub async fn save_flow(db: State<'_, Database>, flow: FlowInput) -> Result<FlowDetail, AppError> {
    let flow = check_flow(flow)?;
    db.with(|conn| repo::save(conn, &flow))
}

#[tauri::command]
pub async fn delete_flow(db: State<'_, Database>, id: String) -> Result<(), AppError> {
    check_id(&id)?;
    db.with(|conn| repo::delete(conn, &id))
}

/// Runs the saved flow; each step's start and result arrive on `on_event` as they happen.
#[tauri::command]
pub async fn run_flow(
    client: State<'_, HttpClient>,
    db: State<'_, Database>,
    vault: State<'_, Vault>,
    runs: State<'_, FlowRuns>,
    id: String,
    run_id: String,
    on_event: Channel<FlowEvent>,
) -> Result<FlowRunSummary, AppError> {
    check_id(&id)?;
    check_id(&run_id)?;
    let flow = db.with(|conn| repo::detail(conn, &id))?;
    if flow.steps.is_empty() {
        return Err(AppError::invalid("Add a step between Start and End first"));
    }
    let stop = Arc::new(AtomicBool::new(false));
    runs.lock().insert(run_id.clone(), Arc::clone(&stop));
    let context = Context {
        client: &client,
        db: &db,
        vault: &vault,
        stop: &stop,
    };
    let summary = run::run(&context, &flow, |event| {
        // A page that was closed meanwhile no longer needs the progress.
        let _ = on_event.send(event);
    })
    .await;
    runs.lock().remove(&run_id);
    Ok(summary)
}

/// Steps already sent finish; the rest are skipped.
#[tauri::command]
pub async fn stop_flow(runs: State<'_, FlowRuns>, run_id: String) -> Result<(), AppError> {
    check_id(&run_id)?;
    if let Some(stop) = runs.lock().get(&run_id) {
        stop.store(true, Ordering::Relaxed);
    }
    Ok(())
}
