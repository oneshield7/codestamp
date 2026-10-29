use std::collections::HashMap;

use rusqlite::Connection;

use super::metrics::{self, RequestLabel, Send, Traffic, Window};
use super::models::{
    CatalogDetail, CatalogEntry, CatalogInfo, CatalogOverview, CatalogSummary, CategoryScores,
    RuleResult, RunBrief, Runtime, Tests,
};
use super::repo;
use super::rules::{self, RequestFacts};
use super::score::{self, percent};
use crate::auth::models::Auth;
use crate::error::AppError;
use crate::storage::models::CollectionSummary;
use crate::storage::repo as collections;

/// Every collection as an API, with the totals above the list.
pub fn overview(
    conn: &Connection,
    now: i64,
    utc_offset_minutes: i32,
) -> Result<CatalogOverview, AppError> {
    let window = Window::ending_today(now, utc_offset_minutes);
    let mut info = repo::all_info(conn)?;
    let auths = repo::collection_auths(conn)?;
    let latest = repo::latest_runs(conn)?;
    let activity = repo::last_activity(conn)?;
    let mut facts: HashMap<String, Vec<RequestFacts>> = HashMap::new();
    for (collection_id, request) in repo::request_facts(conn, None)? {
        facts.entry(collection_id).or_default().push(request);
    }
    let sends = repo::sends(conn, window.previous_start, None)?;
    let mut sends_by: HashMap<&str, Vec<&Send>> = HashMap::new();
    for send in &sends {
        sends_by
            .entry(send.collection_id.as_str())
            .or_default()
            .push(send);
    }

    let mut apis = Vec::new();
    for summary in collections::list_collections(conn)? {
        let requests = facts.remove(&summary.id).unwrap_or_default();
        let traffic = metrics::traffic(
            sends_by.get(summary.id.as_str()).map_or(&[], Vec::as_slice),
            window,
        );
        let rules = rules::check(&requests, auths.get(&summary.id).unwrap_or(&Auth::None));
        let last_activity = activity.get(&summary.id).copied();
        let (entry, _) = entry(Parts {
            info: info.remove(&summary.id).unwrap_or_default(),
            last_run: latest.get(&summary.id).cloned(),
            summary,
            requests: &requests,
            traffic: &traffic,
            rules: &rules,
            last_activity,
        });
        apis.push(entry);
    }
    let traffic = metrics::traffic(&sends.iter().collect::<Vec<_>>(), window);
    Ok(CatalogOverview {
        summary: CatalogSummary {
            apis: apis.len(),
            endpoints: apis.iter().map(|api| api.request_count).sum(),
            requests: traffic.requests,
            errors: traffic.errors,
        },
        tags: tags_in_use(&apis),
        apis,
    })
}

/// One API with its runtime, tests, rules and the flows that use it.
pub fn detail(
    conn: &Connection,
    collection_id: &str,
    now: i64,
    utc_offset_minutes: i32,
) -> Result<CatalogDetail, AppError> {
    let summary = collections::collection_summary(conn, collection_id)?;
    let window = Window::ending_today(now, utc_offset_minutes);
    let requests: Vec<RequestFacts> = repo::request_facts(conn, Some(collection_id))?
        .into_iter()
        .map(|(_, request)| request)
        .collect();
    let sends = repo::sends(conn, window.previous_start, Some(collection_id))?;
    let sends: Vec<&Send> = sends.iter().collect();
    let traffic = metrics::traffic(&sends, window);
    let rules = rules::check(
        &requests,
        &collections::collection_auth(conn, collection_id)?,
    );
    let runs = repo::runs(conn, collection_id)?;
    let flows = repo::flow_uses(conn, collection_id)?;
    let labels: HashMap<&str, RequestLabel> = requests
        .iter()
        .map(|request| {
            (
                request.id.as_str(),
                RequestLabel {
                    name: request.name.clone(),
                    method: request.method.clone(),
                    kind: request.kind,
                },
            )
        })
        .collect();

    let (entry, scores) = entry(Parts {
        info: repo::info(conn, collection_id)?,
        last_run: runs.first().map(|run| run.brief.clone()),
        last_activity: repo::last_activity(conn)?.get(collection_id).copied(),
        summary,
        requests: &requests,
        traffic: &traffic,
        rules: &rules,
    });
    Ok(CatalogDetail {
        scores: score::graded_parts(scores),
        assets: repo::assets(conn, collection_id, flows.len())?,
        runtime: Runtime {
            requests: traffic.requests,
            errors: traffic.errors,
            client_errors: traffic.client_errors,
            error_rate: traffic.error_rate,
            p95_ms: traffic.p95_ms,
            avg_ms: traffic.avg_ms,
            days: metrics::daily(&sends, window),
            endpoints: metrics::top_endpoints(&sends, window, &labels),
        },
        tests: Tests {
            runnable: entry.runnable_count,
            tested: entry.tested_count,
            coverage: percent(entry.tested_count, entry.runnable_count),
            latest: runs.first().cloned(),
            runs: runs.into_iter().map(|run| run.brief).collect(),
        },
        rules,
        flows,
        entry,
    })
}

struct Parts<'a> {
    summary: CollectionSummary,
    info: CatalogInfo,
    requests: &'a [RequestFacts],
    traffic: &'a Traffic,
    rules: &'a [RuleResult],
    last_run: Option<RunBrief>,
    last_activity: Option<i64>,
}

fn entry(parts: Parts<'_>) -> (CatalogEntry, CategoryScores) {
    let count = |keep: fn(&RequestFacts) -> bool| {
        i64::try_from(
            parts
                .requests
                .iter()
                .filter(|request| keep(request))
                .count(),
        )
        .unwrap_or(i64::MAX)
    };
    let runnable_count = count(|request| request.kind.run_skip_reason().is_none());
    let tested_count =
        count(|request| request.kind.run_skip_reason().is_none() && request.has_tests);
    let pass_rate = parts.last_run.as_ref().and_then(|run| {
        percent(
            i64::from(run.passed),
            i64::from(run.passed) + i64::from(run.failed),
        )
    });
    let scores = CategoryScores {
        runtime: score::runtime_score(
            parts.traffic.requests.current,
            parts.traffic.error_rate,
            parts.traffic.p95_ms,
        ),
        tests: score::tests_score(percent(tested_count, runnable_count), pass_rate),
        // An API with no requests has nothing to grade, even if its collection auth passes.
        quality: if parts.requests.is_empty() {
            None
        } else {
            score::quality_score(parts.rules)
        },
    };
    let entry = CatalogEntry {
        id: parts.summary.id,
        name: parts.summary.name,
        icon: parts.summary.icon,
        info: parts.info,
        request_count: parts.summary.request_count,
        runnable_count,
        tested_count,
        example_count: parts.requests.iter().map(|request| request.examples).sum(),
        health: score::health(scores),
        last_run: parts.last_run,
        error_rate: parts.traffic.error_rate,
        p95_ms: parts.traffic.p95_ms,
        last_activity: parts.last_activity,
    };
    (entry, scores)
}

/// Each tag once, whatever its case, in alphabetical order.
fn tags_in_use(apis: &[CatalogEntry]) -> Vec<String> {
    let mut tags: Vec<String> = Vec::new();
    for tag in apis.iter().flat_map(|api| &api.info.tags) {
        if !tags.iter().any(|known| known.eq_ignore_ascii_case(tag)) {
            tags.push(tag.clone());
        }
    }
    tags.sort_by_key(|tag| tag.to_lowercase());
    tags
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::models::{HealthStatus, RunInput, RunOutcome, RunStepInput, StepStatus};
    use crate::storage::db::{Database, now_ms};
    use crate::storage::models::{KeyValue, RequestContent, RequestKind};

    const TODAY_NOON: i64 = 1_760_011_200_000;

    fn content(name: &str, url: &str, test_script: &str) -> RequestContent {
        RequestContent {
            kind: RequestKind::Http,
            name: name.to_owned(),
            method: "GET".to_owned(),
            url: url.to_owned(),
            params: Vec::new(),
            headers: Vec::new(),
            body_mode: "none".to_owned(),
            body: String::new(),
            form: Vec::new(),
            body_file: String::new(),
            pre_request_script: String::new(),
            test_script: test_script.to_owned(),
            auth: Auth::Inherit,
            mcp_operation: None,
            path_variables: Vec::new(),
            graphql_variables: String::new(),
            grpc: None,
            soap: None,
        }
    }

    fn sent(conn: &Connection, collection: &str, request: &str, status: Option<u16>, at: i64) {
        conn.execute(
            "INSERT INTO history (id, collection_id, request_id, name, method, url, params,
                                  headers, body_mode, body, body_omitted, status, error,
                                  elapsed_ms, body_size, created_at)
             VALUES (?1, ?2, ?3, 'x', 'GET', 'u', '[]', '[]', 'none', '', 0, ?4, NULL, 120, 1, ?5)",
            rusqlite::params![
                crate::storage::db::new_id(),
                collection,
                request,
                status,
                at
            ],
        )
        .expect("history row");
    }

    #[test]
    fn builds_the_list_and_one_api_from_history_runs_and_rules() {
        let db = Database::in_memory().expect("database");
        db.with(|conn| {
            let shop = collections::insert_collection(conn, "Shop API")?;
            let empty = collections::insert_collection(conn, "Empty")?;
            let mut login = content("Login", "{{baseUrl}}/login", "pm.test('ok', () => {});");
            login.headers = vec![KeyValue {
                enabled: true,
                key: "Authorization".to_owned(),
                value: "Bearer plain".to_owned(),
            }];
            let login = collections::insert_request(conn, &shop.id, None, login)?;
            let cart = collections::insert_request(
                conn,
                &shop.id,
                None,
                content("Cart", "https://shop.test/cart", ""),
            )?;
            sent(conn, &shop.id, &login.id, Some(200), TODAY_NOON);
            sent(conn, &shop.id, &cart.id, Some(502), TODAY_NOON - 1_000);
            sent(conn, &shop.id, &cart.id, None, TODAY_NOON - 2_000);
            repo::save_info(
                conn,
                &shop.id,
                &CatalogInfo {
                    owner: "Checkout team".to_owned(),
                    tags: vec!["public".to_owned(), "Payments".to_owned()],
                },
            )?;
            let recorded = repo::insert_run(
                conn,
                &RunInput {
                    collection_id: shop.id.clone(),
                    environment: Some(" Staging ".to_owned()),
                    started_at: TODAY_NOON - 10_000,
                    finished_at: TODAY_NOON - 9_000,
                    stopped: false,
                    steps: vec![
                        RunStepInput {
                            request_id: login.id.clone(),
                            status: StepStatus::Passed,
                            code: Some(200),
                            elapsed_ms: Some(100),
                            tests_passed: 1,
                            tests_total: 1,
                        },
                        RunStepInput {
                            request_id: cart.id.clone(),
                            status: StepStatus::Failed,
                            code: Some(500),
                            elapsed_ms: Some(300),
                            tests_passed: 0,
                            tests_total: 0,
                        },
                    ],
                },
            )?;
            assert!(recorded);

            let list = overview(conn, TODAY_NOON, 0)?;
            assert_eq!(list.summary.apis, 2);
            assert_eq!(list.summary.endpoints, 2);
            assert_eq!(list.summary.requests.current, 3);
            assert_eq!(list.summary.errors.current, 2);
            assert_eq!(list.tags, ["Payments", "public"]);
            let shop_row = list
                .apis
                .iter()
                .find(|api| api.id == shop.id)
                .expect("shop");
            assert_eq!(shop_row.info.owner, "Checkout team");
            assert_eq!(shop_row.tested_count, 1);
            assert_eq!(
                shop_row.last_run.as_ref().map(|run| run.outcome),
                Some(RunOutcome::Failed)
            );
            assert!(
                shop_row
                    .error_rate
                    .is_some_and(|rate| (rate - 66.67).abs() < 0.01)
            );
            let empty_row = list
                .apis
                .iter()
                .find(|api| api.id == empty.id)
                .expect("empty");
            assert_eq!(empty_row.health.score, None);
            assert_eq!(empty_row.health.status, HealthStatus::NoData);

            let one = detail(conn, &shop.id, TODAY_NOON, 0)?;
            assert_eq!(one.runtime.days.len(), 7);
            assert_eq!(
                one.runtime.endpoints.first().map(|e| e.name.as_str()),
                Some("Cart")
            );
            assert_eq!(one.tests.coverage, Some(50.0));
            let latest = one.tests.latest.expect("latest run");
            assert_eq!(latest.brief.environment.as_deref(), Some("Staging"));
            assert_eq!((latest.brief.passed, latest.brief.failed), (1, 1));
            assert_eq!(latest.endpoints.len(), 2);
            let secrets = one
                .rules
                .iter()
                .find(|rule| rule.key == "secrets")
                .expect("rule");
            assert_eq!(secrets.findings.len(), 1);
            assert_eq!(one.scores.tests.score, Some(50));
            assert_eq!(one.scores.tests.status, HealthStatus::NeedsAttention);
            assert_ne!(one.entry.health.status, HealthStatus::NoData);
            assert!(one.entry.last_activity.is_some_and(|at| at <= now_ms()));
            Ok(())
        })
        .expect("catalog");
    }

    #[test]
    fn keeps_the_newest_runs_only_and_skips_runs_that_sent_nothing() {
        let db = Database::in_memory().expect("database");
        db.with(|conn| {
            let api = collections::insert_collection(conn, "Users")?;
            let request =
                collections::insert_request(conn, &api.id, None, content("List", "{{u}}", ""))?;
            let run = |at: i64, status: StepStatus| RunInput {
                collection_id: api.id.clone(),
                environment: None,
                started_at: at,
                finished_at: at,
                stopped: false,
                steps: vec![RunStepInput {
                    request_id: request.id.clone(),
                    status,
                    code: None,
                    elapsed_ms: None,
                    tests_passed: 0,
                    tests_total: 0,
                }],
            };
            assert!(!repo::insert_run(conn, &run(1, StepStatus::Skipped))?);
            for at in 1..=25 {
                assert!(repo::insert_run(conn, &run(at, StepStatus::Passed))?);
            }
            let runs = repo::runs(conn, &api.id)?;
            assert_eq!(runs.len(), 20);
            assert_eq!(runs.first().map(|run| run.brief.finished_at), Some(25));
            assert_eq!(runs.last().map(|run| run.brief.finished_at), Some(6));
            Ok(())
        })
        .expect("runs");
    }
}
