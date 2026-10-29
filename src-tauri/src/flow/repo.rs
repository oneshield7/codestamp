use rusqlite::{Connection, OptionalExtension, Row, params};

use super::models::{
    Check, Extract, FlowDetail, FlowInput, FlowStepView, FlowSummary, OnFailure, StepRequest,
};
use crate::environment::lookup::referenced_names;
use crate::error::AppError;
use crate::request::saved::input_from_content;
use crate::storage::db::{new_id, now_ms};
use crate::storage::models::RequestKind;
use crate::storage::repo as collections;

pub fn list(conn: &Connection) -> Result<Vec<FlowSummary>, AppError> {
    let mut statement = conn.prepare(
        "SELECT f.id, f.name, (SELECT COUNT(*) FROM flow_steps s WHERE s.flow_id = f.id)
         FROM flows f ORDER BY f.sort_order, f.created_at",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(FlowSummary {
            id: row.get(0)?,
            name: row.get(1)?,
            step_count: row.get(2)?,
        })
    })?;
    Ok(rows.collect::<Result<_, _>>()?)
}

pub fn insert(conn: &Connection, name: &str) -> Result<FlowSummary, AppError> {
    let id = new_id();
    let now = now_ms();
    conn.execute(
        "INSERT INTO flows (id, name, sort_order, created_at, updated_at)
         VALUES (?1, ?2, (SELECT COALESCE(MAX(sort_order), -1) + 1 FROM flows), ?3, ?3)",
        params![id, name, now],
    )?;
    Ok(FlowSummary {
        id,
        name: name.to_owned(),
        step_count: 0,
    })
}

pub fn delete(conn: &Connection, id: &str) -> Result<(), AppError> {
    let changed = conn.execute("DELETE FROM flows WHERE id = ?1", params![id])?;
    if changed == 0 {
        return Err(AppError::NotFound("Flow"));
    }
    Ok(())
}

pub fn detail(conn: &Connection, id: &str) -> Result<FlowDetail, AppError> {
    let name: String = conn
        .query_row("SELECT name FROM flows WHERE id = ?1", params![id], |row| {
            row.get(0)
        })
        .optional()?
        .ok_or(AppError::NotFound("Flow"))?;
    let mut statement = conn.prepare(
        "SELECT s.id, s.extracts, s.checks, s.on_failure,
                r.id, r.collection_id, c.name, r.kind, r.name, r.method, r.url
         FROM flow_steps s
         LEFT JOIN requests r ON r.id = s.request_id
         LEFT JOIN collections c ON c.id = r.collection_id
         WHERE s.flow_id = ?1
         ORDER BY s.sort_order",
    )?;
    let mut steps: Vec<FlowStepView> = statement
        .query_map(params![id], step_from_row)?
        .collect::<Result<_, _>>()?;
    for request in steps.iter_mut().filter_map(|step| step.request.as_mut()) {
        request.variables = request_variables(conn, &request.id, &request.collection_id);
    }
    Ok(FlowDetail {
        id: id.to_owned(),
        name,
        steps,
    })
}

/// Replaces the flow's name and steps in one transaction. Steps may only call HTTP and GraphQL
/// requests, which are the ones a flow can send.
pub fn save(conn: &Connection, flow: &FlowInput) -> Result<FlowDetail, AppError> {
    let tx = conn.unchecked_transaction()?;
    let changed = tx.execute(
        "UPDATE flows SET name = ?2, updated_at = ?3 WHERE id = ?1",
        params![flow.id, flow.name, now_ms()],
    )?;
    if changed == 0 {
        return Err(AppError::NotFound("Flow"));
    }
    tx.execute(
        "DELETE FROM flow_steps WHERE flow_id = ?1",
        params![flow.id],
    )?;
    for (position, step) in flow.steps.iter().enumerate() {
        if let Some(request_id) = &step.request_id {
            check_runnable(&tx, request_id)?;
        }
        tx.execute(
            "INSERT INTO flow_steps
               (id, flow_id, request_id, extracts, checks, on_failure, sort_order)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                step.id,
                flow.id,
                step.request_id,
                to_json(&step.extracts)?,
                to_json(&step.checks)?,
                step.on_failure.as_str(),
                i64::try_from(position).unwrap_or(i64::MAX),
            ],
        )?;
    }
    tx.commit()?;
    detail(conn, &flow.id)
}

fn check_runnable(conn: &Connection, request_id: &str) -> Result<(), AppError> {
    let kind: Option<String> = conn
        .query_row(
            "SELECT kind FROM requests WHERE id = ?1",
            params![request_id],
            |row| row.get(0),
        )
        .optional()?;
    match kind.as_deref().map(RequestKind::from_stored) {
        None => Err(AppError::NotFound("Request")),
        Some(RequestKind::Http | RequestKind::Graphql | RequestKind::Soap) => Ok(()),
        Some(_) => Err(AppError::invalid(
            "A flow step can call HTTP, GraphQL and SOAP requests",
        )),
    }
}

fn step_from_row(row: &Row<'_>) -> rusqlite::Result<FlowStepView> {
    let extracts: String = row.get(1)?;
    let checks: String = row.get(2)?;
    let on_failure: String = row.get(3)?;
    let request = match row.get::<_, Option<String>>(4)? {
        Some(id) => Some(StepRequest {
            id,
            collection_id: row.get(5)?,
            collection_name: row.get(6)?,
            kind: RequestKind::from_stored(&row.get::<_, String>(7)?),
            name: row.get(8)?,
            method: row.get(9)?,
            url: row.get(10)?,
            variables: Vec::new(),
        }),
        None => None,
    };
    Ok(FlowStepView {
        id: row.get(0)?,
        request,
        // Rules that no longer read (edited by hand, say) are left out rather than failing.
        extracts: serde_json::from_str::<Vec<Extract>>(&extracts).unwrap_or_default(),
        checks: serde_json::from_str::<Vec<Check>>(&checks).unwrap_or_default(),
        on_failure: OnFailure::from_stored(&on_failure),
    })
}

/// A request whose saved content no longer reads lists nothing; running the step reports why.
fn request_variables(conn: &Connection, request_id: &str, collection_id: &str) -> Vec<String> {
    collections::load_request(conn, request_id)
        .map(|saved| {
            let input = input_from_content(&saved.content, collection_id, request_id);
            referenced_names(&input).into_iter().collect()
        })
        .unwrap_or_default()
}

fn to_json<T: serde::Serialize>(value: &T) -> Result<String, AppError> {
    serde_json::to_string(value).map_err(|_| AppError::Storage)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::models::Auth;
    use crate::flow::models::{CheckOp, CheckSubject, ExtractSource, FlowStepInput};
    use crate::storage::db::Database;
    use crate::storage::models::{KeyValue, RequestContent};

    fn content(kind: RequestKind, name: &str) -> RequestContent {
        RequestContent {
            kind,
            name: name.to_owned(),
            method: "POST".to_owned(),
            url: "{{baseUrl}}/register".to_owned(),
            params: Vec::new(),
            headers: Vec::new(),
            body_mode: "none".to_owned(),
            body: String::new(),
            form: Vec::new(),
            body_file: String::new(),
            pre_request_script: String::new(),
            test_script: String::new(),
            auth: Auth::Inherit,
            mcp_operation: None,
            path_variables: Vec::new(),
            graphql_variables: String::new(),
            grpc: None,
            soap: None,
        }
    }

    #[test]
    fn saves_steps_in_order_and_keeps_a_gap_for_deleted_requests() {
        let db = Database::in_memory().expect("database");
        db.with(|conn| {
            let collection = collections::insert_collection(conn, "Shop")?;
            let register = collections::insert_request(
                conn,
                &collection.id,
                None,
                content(RequestKind::Http, "Register"),
            )?;
            let socket = collections::insert_request(
                conn,
                &collection.id,
                None,
                content(RequestKind::Websocket, "Live"),
            )?;
            let flow = insert(conn, "Register to checkout")?;
            let step = |id: &str, request_id: &str| FlowStepInput {
                id: id.to_owned(),
                request_id: Some(request_id.to_owned()),
                extracts: vec![Extract {
                    name: "userId".to_owned(),
                    source: ExtractSource::Body,
                    path: "id".to_owned(),
                }],
                checks: vec![Check {
                    subject: CheckSubject::Status,
                    path: String::new(),
                    op: CheckOp::Equals,
                    value: "201".to_owned(),
                }],
                on_failure: OnFailure::Continue,
            };
            let saved = save(
                conn,
                &FlowInput {
                    id: flow.id.clone(),
                    name: "Signup".to_owned(),
                    steps: vec![step(&new_id(), &register.id), step(&new_id(), &register.id)],
                },
            )?;
            assert_eq!(saved.name, "Signup");
            assert_eq!(saved.steps.len(), 2);
            assert_eq!(saved.steps[0].extracts[0].name, "userId");
            assert_eq!(saved.steps[1].on_failure, OnFailure::Continue);
            assert_eq!(list(conn)?[0].step_count, 2);

            let refused = save(
                conn,
                &FlowInput {
                    id: flow.id.clone(),
                    name: "Signup".to_owned(),
                    steps: vec![step(&new_id(), &socket.id)],
                },
            );
            assert!(refused.is_err());
            assert_eq!(
                detail(conn, &flow.id)?.steps.len(),
                2,
                "a refused save changes nothing"
            );

            assert_eq!(
                saved.steps[0]
                    .request
                    .as_ref()
                    .map(|request| request.variables.clone()),
                Some(vec!["baseUrl".to_owned()])
            );

            collections::delete_request(conn, &register.id)?;
            let after = detail(conn, &flow.id)?;
            assert!(after.steps.iter().all(|step| step.request.is_none()));
            delete(conn, &flow.id)?;
            assert!(list(conn)?.is_empty());
            Ok(())
        })
        .expect("flow storage");
    }

    #[test]
    fn lists_the_variables_each_step_sends() {
        let db = Database::in_memory().expect("database");
        db.with(|conn| {
            let collection = collections::insert_collection(conn, "Shop")?;
            let mut checkout = content(RequestKind::Http, "Checkout");
            checkout.url = "{{baseUrl}}/carts/{{cartId}}/checkout".to_owned();
            checkout.headers = vec![
                KeyValue {
                    enabled: true,
                    key: "Authorization".to_owned(),
                    value: "Bearer {{token}}".to_owned(),
                },
                KeyValue {
                    enabled: false,
                    key: "X-Trace".to_owned(),
                    value: "{{traceId}}".to_owned(),
                },
            ];
            checkout.body_mode = "json".to_owned();
            checkout.body = r#"{ "coupon": "{{coupon}}", "note": "{{ not a name }}" }"#.to_owned();
            let request = collections::insert_request(conn, &collection.id, None, checkout)?;
            let flow = insert(conn, "Checkout")?;
            let saved = save(
                conn,
                &FlowInput {
                    id: flow.id,
                    name: "Checkout".to_owned(),
                    steps: vec![FlowStepInput {
                        id: new_id(),
                        request_id: Some(request.id),
                        extracts: Vec::new(),
                        checks: Vec::new(),
                        on_failure: OnFailure::Stop,
                    }],
                },
            )?;
            let variables = saved.steps[0]
                .request
                .as_ref()
                .map(|request| request.variables.clone());
            assert_eq!(
                variables,
                Some(vec![
                    "baseUrl".to_owned(),
                    "cartId".to_owned(),
                    "coupon".to_owned(),
                    "token".to_owned(),
                ]),
                "a disabled header is not sent, so its variable is left out"
            );
            Ok(())
        })
        .expect("flow storage");
    }
}
