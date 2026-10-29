use super::models::{CategoryHealth, CategoryScores, Health, HealthStatus, RuleResult, Severity};

// The catalog page explains these weights and bands to the user; keep its text in step.
const RUNTIME_WEIGHT: f64 = 40.0;
const TESTS_WEIGHT: f64 = 35.0;
const QUALITY_WEIGHT: f64 = 25.0;
const HEALTHY_FROM: u8 = 80;
const ATTENTION_FROM: u8 = 50;
/// A broken rule of `Error` severity counts three times as much as a warning.
const ERROR_RULE_WEIGHT: f64 = 3.0;

/// `part` of `whole` in percent, or `None` when there is nothing to divide.
pub fn percent(part: i64, whole: i64) -> Option<f64> {
    (whole > 0).then(|| part as f64 * 100.0 / whole as f64)
}

/// Mostly the error rate, partly how slow the slowest 5% of answers were.
pub fn runtime_score(requests: i64, error_rate: Option<f64>, p95_ms: Option<i64>) -> Option<u8> {
    if requests == 0 {
        return None;
    }
    let errors = error_rate.map_or(0.0, error_points);
    let latency = p95_ms.map_or(0.0, latency_points);
    Some(points(errors.mul_add(0.7, latency * 0.3)))
}

fn error_points(rate: f64) -> f64 {
    match rate {
        rate if rate <= 1.0 => 100.0,
        rate if rate <= 5.0 => 75.0,
        rate if rate <= 10.0 => 50.0,
        rate if rate <= 25.0 => 25.0,
        _ => 0.0,
    }
}

fn latency_points(p95_ms: i64) -> f64 {
    match p95_ms {
        ..=500 => 100.0,
        501..=1_000 => 80.0,
        1_001..=2_000 => 60.0,
        2_001..=5_000 => 30.0,
        _ => 10.0,
    }
}

/// The mean of test coverage and the latest run's pass rate, whichever exist (both percent).
pub fn tests_score(coverage: Option<f64>, pass_rate: Option<f64>) -> Option<u8> {
    let parts: Vec<f64> = [coverage, pass_rate].into_iter().flatten().collect();
    if parts.is_empty() {
        return None;
    }
    Some(points(parts.iter().sum::<f64>() / parts.len() as f64))
}

/// The share of rule checks that pass, with error rules weighing more than warnings.
pub fn quality_score(rules: &[RuleResult]) -> Option<u8> {
    let (mut passed, mut checked) = (0.0, 0.0);
    for rule in rules {
        let weight = match rule.severity {
            Severity::Error => ERROR_RULE_WEIGHT,
            Severity::Warning => 1.0,
        };
        let failing = failing_items(rule);
        checked += weight * rule.checked as f64;
        passed += weight * rule.checked.saturating_sub(failing) as f64;
    }
    (checked > 0.0).then(|| points(passed * 100.0 / checked))
}

/// A request that breaks a rule twice (two plain-text secrets) still counts as one failing check.
fn failing_items(rule: &RuleResult) -> usize {
    let mut seen: Vec<Option<&str>> = Vec::new();
    for finding in &rule.findings {
        let item = finding.request.as_ref().map(|request| request.id.as_str());
        if !seen.contains(&item) {
            seen.push(item);
        }
    }
    seen.len()
}

/// The weighted mean of the parts that have something to measure.
pub fn health(scores: CategoryScores) -> Health {
    let parts = [
        (scores.runtime, RUNTIME_WEIGHT),
        (scores.tests, TESTS_WEIGHT),
        (scores.quality, QUALITY_WEIGHT),
    ];
    let (mut total, mut weights) = (0.0, 0.0);
    for (score, weight) in parts {
        if let Some(score) = score {
            total += f64::from(score) * weight;
            weights += weight;
        }
    }
    graded((weights > 0.0).then(|| points(total / weights)))
}

pub fn graded_parts(scores: CategoryScores) -> CategoryHealth {
    CategoryHealth {
        runtime: graded(scores.runtime),
        tests: graded(scores.tests),
        quality: graded(scores.quality),
    }
}

fn graded(score: Option<u8>) -> Health {
    let status = match score {
        None => HealthStatus::NoData,
        Some(score) if score >= HEALTHY_FROM => HealthStatus::Healthy,
        Some(score) if score >= ATTENTION_FROM => HealthStatus::NeedsAttention,
        Some(_) => HealthStatus::Critical,
    };
    Health { score, status }
}

fn points(value: f64) -> u8 {
    // Clamped to 0..=100 first, so the cast cannot truncate.
    value.round().clamp(0.0, 100.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::models::{Finding, FindingRequest};
    use crate::storage::models::RequestKind;

    fn rule(severity: Severity, checked: usize, failing_ids: &[&str]) -> RuleResult {
        RuleResult {
            key: "rule",
            title: "Rule",
            description: "",
            severity,
            checked,
            findings: failing_ids
                .iter()
                .map(|id| Finding {
                    request: Some(FindingRequest {
                        id: (*id).to_owned(),
                        name: (*id).to_owned(),
                        method: "GET".to_owned(),
                        kind: RequestKind::Http,
                    }),
                    detail: String::new(),
                })
                .collect(),
        }
    }

    #[test]
    fn runtime_needs_traffic_and_weighs_errors_most() {
        assert_eq!(runtime_score(0, None, None), None);
        assert_eq!(runtime_score(10, Some(0.0), Some(120)), Some(100));
        // 8% errors (50 points) and a 1.5 s p95 (60 points).
        assert_eq!(runtime_score(50, Some(8.0), Some(1_500)), Some(53));
        // Nothing answered: no latency to score.
        assert_eq!(runtime_score(3, Some(100.0), None), Some(0));
    }

    #[test]
    fn tests_use_whatever_was_measured() {
        assert_eq!(tests_score(None, None), None);
        assert_eq!(tests_score(Some(50.0), None), Some(50));
        assert_eq!(tests_score(Some(50.0), Some(100.0)), Some(75));
    }

    #[test]
    fn quality_counts_a_request_once_per_rule_and_errors_triple() {
        assert_eq!(quality_score(&[]), None);
        // Error rule: 1 of 4 fails (twice); warning rule: 2 of 4 fail.
        let rules = [
            rule(Severity::Error, 4, &["a", "a"]),
            rule(Severity::Warning, 4, &["b", "c"]),
        ];
        // (3 * 3 + 2) / (3 * 4 + 4) = 11 / 16.
        assert_eq!(quality_score(&rules), Some(69));
    }

    #[test]
    fn health_weighs_the_parts_that_exist() {
        let empty = health(CategoryScores::default());
        assert_eq!(empty.status, HealthStatus::NoData);
        assert_eq!(empty.score, None);

        let quality_only = health(CategoryScores {
            quality: Some(64),
            ..CategoryScores::default()
        });
        assert_eq!(quality_only.score, Some(64));
        assert_eq!(quality_only.status, HealthStatus::NeedsAttention);

        let all = health(CategoryScores {
            runtime: Some(100),
            tests: Some(40),
            quality: Some(80),
        });
        // (100 * 40 + 40 * 35 + 80 * 25) / 100.
        assert_eq!(all.score, Some(74));
        assert_eq!(
            health(CategoryScores {
                runtime: Some(20),
                ..CategoryScores::default()
            })
            .status,
            HealthStatus::Critical
        );
    }
}
