use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, Row, params};

use super::metrics::Send;
use super::models::{
    Assets, CatalogInfo, FlowUse, KindCount, RunBrief, RunEndpoint, RunInput, RunOutcome,
    RunRecord, StepStatus,
};
use super::rules::RequestFacts;
use crate::auth::models::Auth;
use crate::error::AppError;
use crate::storage::db::{new_id, now_ms};
use crate::storage::models::RequestKind;
use crate::storage::repo::{from_json, has_collection, to_json};

/// Runs kept per collection; older ones are removed when a new one is saved.
const KEPT_RUNS: i64 = 20;
const KIND_ORDER: [RequestKind; 6] = [
    RequestKind::Http,
    RequestKind::Graphql,
    RequestKind::Soap,
    RequestKind::Websocket,
    RequestKind::Grpc,
    RequestKind::Mcp,
];

pub fn all_info(conn: &Connection) -> Result<HashMap<String, CatalogInfo>, AppError> {
    let mut statement = conn.prepare("SELECT collection_id, owner, tags FROM catalog_info")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    let mut info = HashMap::new();
    for row in rows {
        let (id, owner, tags) = row?;
        info.insert(
            id,
            CatalogInfo {
                owner,
                tags: from_json(&tags)?,
            },
        );
    }
    Ok(info)
}

/// An empty owner and no tags leave no row behind.
pub fn save_info(conn: &Connection, id: &str, info: &CatalogInfo) -> Result<(), AppError> {
    if !has_collection(conn, id)? {
        return Err(AppError::NotFound("Collection"));
    }
    if info.owner.is_empty() && info.tags.is_empty() {
        conn.execute(
            "DELETE FROM catalog_info WHERE collection_id = ?1",
            params![id],
        )?;
        return Ok(());
    }
    conn.execute(
        "INSERT INTO catalog_info (collection_id, owner, tags, updated_at)
         VALUES (?1, ?2, ?3, ?4)
         ON CONFLICT(collection_id) DO UPDATE
           SET owner = excluded.owner, tags = excluded.tags, updated_at = excluded.updated_at",
        params![id, info.owner, to_json(&info.tags)?, now_ms()],
    )?;
    Ok(())
}

/// Saved requests with what the quality rules look at, by collection, in notebook order.
pub fn request_facts(
    conn: &Connection,
    collection_id: Option<&str>,
) -> Result<Vec<(String, RequestFacts)>, AppError> {
    let mut statement = conn.prepare(
        "SELECT r.id, r.collection_id, r.name, r.method, r.url, r.kind, r.params, r.headers,
                r.auth, length(trim(r.test_script)) > 0,
                (SELECT COUNT(*) FROM request_examples e WHERE e.request_id = r.id)
         FROM requests r
         WHERE ?1 IS NULL OR r.collection_id = ?1
         ORDER BY r.collection_id, r.sort_order",
    )?;
    let rows = statement.query_map(params![collection_id], |row| {
        Ok((
            row.get::<_, String>(1)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, String>(8)?,
            RequestFacts {
                id: row.get(0)?,
                name: row.get(2)?,
                method: row.get(3)?,
                url: row.get(4)?,
                kind: RequestKind::from_stored(&row.get::<_, String>(5)?),
                params: Vec::new(),
                headers: Vec::new(),
                auth: Auth::Inherit,
                has_tests: row.get(9)?,
                examples: row.get(10)?,
            },
        ))
    })?;
    let mut facts = Vec::new();
    for row in rows {
        let (collection, params, headers, auth, mut request) = row?;
        request.params = from_json(&params)?;
        request.headers = from_json(&headers)?;
        request.auth = Auth::from_stored(&auth, Auth::Inherit);
        facts.push((collection, request));
    }
    Ok(facts)
}

pub fn collection_auths(conn: &Connection) -> Result<HashMap<String, Auth>, AppError> {
    let mut statement = conn.prepare("SELECT id, auth FROM collections")?;
    let rows = statement.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            Auth::from_stored(&row.get::<_, String>(1)?, Auth::None),
        ))
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// History entries of requests sent from a collection since `since`.
pub fn sends(
    conn: &Connection,
    since: i64,
    collection_id: Option<&str>,
) -> Result<Vec<Send>, AppError> {
    let mut statement = conn.prepare(
        "SELECT collection_id, request_id, status, elapsed_ms, created_at FROM history
         WHERE created_at >= ?1 AND collection_id IS NOT NULL
           AND (?2 IS NULL OR collection_id = ?2)",
    )?;
    let rows = statement.query_map(params![since, collection_id], |row| {
        Ok(Send {
            collection_id: row.get(0)?,
            request_id: row.get(1)?,
            status: row.get(2)?,
            elapsed_ms: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// Saves a finished Run all, one row per request. Requests deleted while the run was going are
/// left out, since there is nothing left to show them by. Returns false when nothing was sent.
pub fn insert_run(conn: &Connection, run: &RunInput) -> Result<bool, AppError> {
    if !has_collection(conn, &run.collection_id)? {
        return Err(AppError::NotFound("Collection"));
    }
    let labels: HashMap<String, (String, String, RequestKind)> = {
        let mut statement =
            conn.prepare("SELECT id, name, method, kind FROM requests WHERE collection_id = ?1")?;
        let rows = statement.query_map(params![run.collection_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                (
                    row.get(1)?,
                    row.get(2)?,
                    RequestKind::from_stored(&row.get::<_, String>(3)?),
                ),
            ))
        })?;
        rows.collect::<Result<_, _>>()?
    };
    let endpoints = add_up(run, &labels);
    let sent = endpoints
        .iter()
        .any(|endpoint| endpoint.passed + endpoint.failed > 0);
    if !sent {
        return Ok(false);
    }
    let outcome = if run.stopped {
        RunOutcome::Stopped
    } else if endpoints.iter().any(|endpoint| endpoint.failed > 0) {
        RunOutcome::Failed
    } else {
        RunOutcome::Passed
    };
    let environment = run
        .environment
        .as_deref()
        .map(str::trim)
        .filter(|name| !name.is_empty());
    conn.execute(
        "INSERT INTO collection_runs
           (id, collection_id, environment, started_at, finished_at, outcome, results)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            new_id(),
            run.collection_id,
            environment,
            run.started_at,
            run.finished_at,
            outcome.as_str(),
            to_json(&endpoints)?,
        ],
    )?;
    conn.execute(
        "DELETE FROM collection_runs WHERE collection_id = ?1 AND id NOT IN
           (SELECT id FROM collection_runs WHERE collection_id = ?1
            ORDER BY finished_at DESC LIMIT ?2)",
        params![run.collection_id, KEPT_RUNS],
    )?;
    Ok(true)
}

/// Iterations of the same request become one row, in the order the run met them.
fn add_up(
    run: &RunInput,
    labels: &HashMap<String, (String, String, RequestKind)>,
) -> Vec<RunEndpoint> {
    let mut endpoints: Vec<RunEndpoint> = Vec::new();
    let mut times: HashMap<String, (u64, u64)> = HashMap::new();
    for step in &run.steps {
        let Some((name, method, kind)) = labels.get(&step.request_id) else {
            continue;
        };
        let position = match endpoints
            .iter()
            .position(|endpoint| endpoint.request_id == step.request_id)
        {
            Some(position) => position,
            None => {
                endpoints.push(RunEndpoint {
                    request_id: step.request_id.clone(),
                    name: name.clone(),
                    method: method.clone(),
                    kind: *kind,
                    passed: 0,
                    failed: 0,
                    skipped: 0,
                    tests_passed: 0,
                    tests_total: 0,
                    avg_ms: None,
                    last_code: None,
                });
                endpoints.len() - 1
            }
        };
        let Some(endpoint) = endpoints.get_mut(position) else {
            continue;
        };
        match step.status {
            StepStatus::Passed => endpoint.passed += 1,
            StepStatus::Failed => endpoint.failed += 1,
            StepStatus::Skipped => endpoint.skipped += 1,
        }
        endpoint.tests_passed = endpoint.tests_passed.saturating_add(step.tests_passed);
        endpoint.tests_total = endpoint.tests_total.saturating_add(step.tests_total);
        if step.code.is_some() {
            endpoint.last_code = step.code;
        }
        if let Some(elapsed) = step.elapsed_ms {
            let (total, count) = times.entry(step.request_id.clone()).or_default();
            *total = total.saturating_add(elapsed);
            *count += 1;
            endpoint.avg_ms = Some(*total / *count);
        }
    }
    endpoints
}

/// The newest runs of a collection, newest first.
pub fn runs(conn: &Connection, collection_id: &str) -> Result<Vec<RunRecord>, AppError> {
    let mut statement = conn.prepare(
        "SELECT id, environment, started_at, finished_at, outcome, results FROM collection_runs
         WHERE collection_id = ?1 ORDER BY finished_at DESC LIMIT ?2",
    )?;
    let rows = statement.query_map(params![collection_id, KEPT_RUNS], run_row)?;
    rows.map(|row| run_record(row?)).collect()
}

/// The newest run of every collection that has one.
pub fn latest_runs(conn: &Connection) -> Result<HashMap<String, RunBrief>, AppError> {
    let mut statement = conn.prepare(
        "SELECT r.id, r.environment, r.started_at, r.finished_at, r.outcome, r.results,
                r.collection_id
         FROM collection_runs r
         WHERE r.id = (SELECT n.id FROM collection_runs n WHERE n.collection_id = r.collection_id
                       ORDER BY n.finished_at DESC LIMIT 1)",
    )?;
    let rows = statement.query_map([], |row| Ok((run_row(row)?, row.get::<_, String>(6)?)))?;
    let mut latest = HashMap::new();
    for row in rows {
        let (stored, collection_id) = row?;
        latest.insert(collection_id, run_record(stored)?.brief);
    }
    Ok(latest)
}

type StoredRun = (String, Option<String>, i64, i64, String, String);

fn run_row(row: &Row<'_>) -> rusqlite::Result<StoredRun> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

fn run_record(
    (id, environment, started_at, finished_at, outcome, results): StoredRun,
) -> Result<RunRecord, AppError> {
    let endpoints: Vec<RunEndpoint> = from_json(&results)?;
    let sum = |field: fn(&RunEndpoint) -> u32| {
        endpoints.iter().fold(0_u32, |total, endpoint| {
            total.saturating_add(field(endpoint))
        })
    };
    Ok(RunRecord {
        brief: RunBrief {
            id,
            environment,
            started_at,
            finished_at,
            outcome: RunOutcome::from_stored(&outcome),
            passed: sum(|endpoint| endpoint.passed),
            failed: sum(|endpoint| endpoint.failed),
            tests_passed: sum(|endpoint| endpoint.tests_passed),
            tests_total: sum(|endpoint| endpoint.tests_total),
        },
        endpoints,
    })
}

pub fn assets(conn: &Connection, collection_id: &str, flows: usize) -> Result<Assets, AppError> {
    let mut statement =
        conn.prepare("SELECT kind, COUNT(*) FROM requests WHERE collection_id = ?1 GROUP BY kind")?;
    let rows = statement.query_map(params![collection_id], |row| {
        Ok((
            RequestKind::from_stored(&row.get::<_, String>(0)?),
            row.get::<_, i64>(1)?,
        ))
    })?;
    let mut counts: HashMap<RequestKind, i64> = HashMap::new();
    for row in rows {
        let (kind, count) = row?;
        *counts.entry(kind).or_default() += count;
    }
    let count = |sql: &str| -> Result<i64, AppError> {
        Ok(conn.query_row(sql, params![collection_id], |row| row.get(0))?)
    };
    Ok(Assets {
        kinds: KIND_ORDER
            .iter()
            .filter_map(|kind| {
                counts.get(kind).map(|count| KindCount {
                    kind: *kind,
                    count: *count,
                })
            })
            .collect(),
        sections: count("SELECT COUNT(*) FROM sections WHERE collection_id = ?1")?,
        examples: count(
            "SELECT COUNT(*) FROM request_examples e JOIN requests r ON r.id = e.request_id
             WHERE r.collection_id = ?1",
        )?,
        variables: count(
            "SELECT COUNT(*) FROM variables v JOIN environments e ON e.id = v.environment_id
             WHERE e.collection_id = ?1",
        )?,
        flows: i64::try_from(flows).unwrap_or(i64::MAX),
    })
}

/// Flows with at least one step from this collection, in Orchestration's order.
pub fn flow_uses(conn: &Connection, collection_id: &str) -> Result<Vec<FlowUse>, AppError> {
    let mut statement = conn.prepare(
        "SELECT f.id, f.name, r.collection_id, c.name
         FROM flows f
         JOIN flow_steps s ON s.flow_id = f.id
         LEFT JOIN requests r ON r.id = s.request_id
         LEFT JOIN collections c ON c.id = r.collection_id
         WHERE f.id IN (SELECT s2.flow_id FROM flow_steps s2
                        JOIN requests r2 ON r2.id = s2.request_id
                        WHERE r2.collection_id = ?1)
         ORDER BY f.sort_order, f.created_at, s.sort_order",
    )?;
    let rows = statement.query_map(params![collection_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, Option<String>>(3)?,
        ))
    })?;
    let mut uses: Vec<FlowUse> = Vec::new();
    for row in rows {
        let (flow_id, flow_name, step_collection, step_collection_name) = row?;
        if uses.last().is_none_or(|last| last.id != flow_id) {
            uses.push(FlowUse {
                id: flow_id,
                name: flow_name,
                steps_here: 0,
                steps_total: 0,
                other_apis: Vec::new(),
            });
        }
        let Some(flow) = uses.last_mut() else {
            continue;
        };
        flow.steps_total += 1;
        if step_collection.as_deref() == Some(collection_id) {
            flow.steps_here += 1;
        } else if let Some(name) = step_collection_name
            && !flow.other_apis.contains(&name)
        {
            flow.other_apis.push(name);
        }
    }
    Ok(uses)
}

/// When each collection last changed, was sent from or was run.
pub fn last_activity(conn: &Connection) -> Result<HashMap<String, i64>, AppError> {
    let mut statement = conn.prepare(
        "SELECT c.id, MAX(c.updated_at,
                   COALESCE((SELECT MAX(r.updated_at) FROM requests r
                             WHERE r.collection_id = c.id), 0),
                   COALESCE((SELECT MAX(h.created_at) FROM history h
                             WHERE h.collection_id = c.id), 0),
                   COALESCE((SELECT MAX(n.finished_at) FROM collection_runs n
                             WHERE n.collection_id = c.id), 0))
         FROM collections c",
    )?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?;
    Ok(rows.collect::<Result<_, _>>()?)
}

/// The owner and tags of one API; none saved yet reads as empty.
pub fn info(conn: &Connection, collection_id: &str) -> Result<CatalogInfo, AppError> {
    let stored: Option<(String, String)> = conn
        .query_row(
            "SELECT owner, tags FROM catalog_info WHERE collection_id = ?1",
            params![collection_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()?;
    stored.map_or_else(
        || Ok(CatalogInfo::default()),
        |(owner, tags)| {
            Ok(CatalogInfo {
                owner,
                tags: from_json(&tags)?,
            })
        },
    )
}
