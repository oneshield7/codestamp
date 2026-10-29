use std::collections::HashMap;

use super::models::{DayTraffic, EndpointTraffic, Trend};
use super::score::percent;
use crate::storage::models::RequestKind;

const DAY_MS: i64 = 24 * 60 * 60 * 1000;
/// The catalog looks at the last 7 local days, today included, like Postman's API Catalog.
const DAYS: i64 = 7;
const TOP_ENDPOINTS: usize = 5;

/// One history entry of a request sent from a collection.
pub struct Send {
    pub collection_id: String,
    pub request_id: Option<String>,
    pub status: Option<u16>,
    pub elapsed_ms: Option<i64>,
    pub created_at: i64,
}

impl Send {
    /// No answer at all, or a server error. A 4xx is the caller's mistake, counted apart.
    const fn is_error(&self) -> bool {
        match self.status {
            None => true,
            Some(status) => status >= 500,
        }
    }

    const fn is_client_error(&self) -> bool {
        matches!(self.status, Some(400..=499))
    }
}

/// The 7 local days ending today, and the 7 days before them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    pub start: i64,
    pub previous_start: i64,
}

impl Window {
    /// `utc_offset_minutes` is how far the user's clock is ahead of UTC (420 for UTC+7).
    pub fn ending_today(now: i64, utc_offset_minutes: i32) -> Self {
        let offset = i64::from(utc_offset_minutes) * 60 * 1000;
        let today = (now + offset).div_euclid(DAY_MS) * DAY_MS - offset;
        let start = today - (DAYS - 1) * DAY_MS;
        Self {
            start,
            previous_start: start - DAYS * DAY_MS,
        }
    }

    fn holds(self, send: &Send) -> bool {
        send.created_at >= self.start
    }
}

/// Totals for a window of sends.
pub struct Traffic {
    pub requests: Trend,
    pub errors: Trend,
    pub client_errors: i64,
    pub error_rate: Option<f64>,
    pub p95_ms: Option<i64>,
    pub avg_ms: Option<i64>,
}

pub fn traffic(sends: &[&Send], window: Window) -> Traffic {
    let (current, previous): (Vec<&Send>, Vec<&Send>) = sends
        .iter()
        .filter(|send| send.created_at >= window.previous_start)
        .partition(|send| window.holds(send));
    let mut times: Vec<i64> = current.iter().filter_map(|send| send.elapsed_ms).collect();
    let requests = Trend {
        current: count(&current, |_| true),
        previous: count(&previous, |_| true),
    };
    let errors = Trend {
        current: count(&current, Send::is_error),
        previous: count(&previous, Send::is_error),
    };
    Traffic {
        requests,
        errors,
        client_errors: count(&current, Send::is_client_error),
        error_rate: percent(errors.current, requests.current),
        avg_ms: average(&times),
        p95_ms: p95(&mut times),
    }
}

/// Sends and errors per local day, oldest first.
pub fn daily(sends: &[&Send], window: Window) -> Vec<DayTraffic> {
    (0..DAYS)
        .map(|day| {
            let start = window.start + day * DAY_MS;
            let on_day: Vec<&Send> = sends
                .iter()
                .copied()
                .filter(|send| (start..start + DAY_MS).contains(&send.created_at))
                .collect();
            DayTraffic {
                start,
                requests: count(&on_day, |_| true),
                errors: count(&on_day, Send::is_error),
            }
        })
        .collect()
}

fn count(sends: &[&Send], keep: impl Fn(&Send) -> bool) -> i64 {
    i64::try_from(sends.iter().filter(|send| keep(send)).count()).unwrap_or(i64::MAX)
}

/// How a request is shown: its current name, method and kind.
pub struct RequestLabel {
    pub name: String,
    pub method: String,
    pub kind: RequestKind,
}

/// The saved requests with the highest error rate in the window; more sends break ties.
pub fn top_endpoints(
    sends: &[&Send],
    window: Window,
    labels: &HashMap<&str, RequestLabel>,
) -> Vec<EndpointTraffic> {
    let mut by_request: HashMap<&str, Vec<&Send>> = HashMap::new();
    for send in sends.iter().filter(|send| window.holds(send)) {
        if let Some(id) = send.request_id.as_deref() {
            by_request.entry(id).or_default().push(send);
        }
    }
    let mut endpoints: Vec<EndpointTraffic> = by_request
        .into_iter()
        .filter_map(|(id, sends)| {
            let label = labels.get(id)?;
            let requests = count(&sends, |_| true);
            let errors = count(&sends, Send::is_error);
            let mut times: Vec<i64> = sends.iter().filter_map(|send| send.elapsed_ms).collect();
            Some(EndpointTraffic {
                request_id: id.to_owned(),
                name: label.name.clone(),
                method: label.method.clone(),
                kind: label.kind,
                requests,
                errors,
                error_rate: percent(errors, requests).unwrap_or(0.0),
                p95_ms: p95(&mut times),
            })
        })
        .collect();
    endpoints.sort_by(|a, b| {
        b.error_rate
            .total_cmp(&a.error_rate)
            .then(b.requests.cmp(&a.requests))
            .then(a.name.cmp(&b.name))
    });
    endpoints.truncate(TOP_ENDPOINTS);
    endpoints
}

/// The nearest-rank 95th percentile: 95% of the answers came at least this fast.
fn p95(times: &mut [i64]) -> Option<i64> {
    if times.is_empty() {
        return None;
    }
    times.sort_unstable();
    let rank = (times.len() * 95).div_ceil(100);
    times.get(rank.saturating_sub(1)).copied()
}

fn average(times: &[i64]) -> Option<i64> {
    let count = i64::try_from(times.len()).ok().filter(|count| *count > 0)?;
    Some(times.iter().sum::<i64>() / count)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_760_000_000_000;

    fn send(request: &str, status: Option<u16>, elapsed: i64, at: i64) -> Send {
        Send {
            collection_id: "c".to_owned(),
            request_id: Some(request.to_owned()),
            status,
            elapsed_ms: status.map(|_| elapsed),
            created_at: at,
        }
    }

    #[test]
    fn the_window_starts_at_local_midnight_six_days_ago() {
        // 2025-10-09 08:53:20 UTC is 15:53:20 in UTC+7.
        let window = Window::ending_today(NOW, 420);
        let local_midnight = (NOW + 420 * 60_000).div_euclid(DAY_MS) * DAY_MS - 420 * 60_000;
        assert_eq!(window.start, local_midnight - 6 * DAY_MS);
        assert_eq!(window.previous_start, window.start - 7 * DAY_MS);
        assert_eq!((window.start + 420 * 60_000).rem_euclid(DAY_MS), 0);
    }

    #[test]
    fn counts_errors_client_errors_and_the_week_before() {
        let window = Window::ending_today(NOW, 0);
        let sends = [
            send("a", Some(200), 100, NOW),
            send("a", Some(503), 900, NOW - 1_000),
            send("b", None, 0, NOW - 2_000),
            send("b", Some(404), 50, NOW - 3_000),
            send("a", Some(200), 80, window.start - DAY_MS),
            send("a", Some(200), 80, window.previous_start - 1),
        ];
        let refs: Vec<&Send> = sends.iter().collect();
        let totals = traffic(&refs, window);
        assert_eq!(
            totals.requests,
            Trend {
                current: 4,
                previous: 1
            }
        );
        assert_eq!(
            totals.errors,
            Trend {
                current: 2,
                previous: 0
            }
        );
        assert_eq!(totals.client_errors, 1);
        assert_eq!(totals.error_rate, Some(50.0));
        assert_eq!(totals.p95_ms, Some(900));
        assert_eq!(totals.avg_ms, Some(350));

        let days = daily(&refs, window);
        assert_eq!(days.len(), 7);
        assert_eq!(days[6].requests, 4);
        assert_eq!(days[6].errors, 2);
        assert_eq!(days[..6].iter().map(|day| day.requests).sum::<i64>(), 0);
    }

    #[test]
    fn ranks_endpoints_by_error_rate() {
        let window = Window::ending_today(NOW, 0);
        let sends = [
            send("ok", Some(200), 10, NOW),
            send("ok", Some(200), 10, NOW),
            send("flaky", Some(200), 10, NOW),
            send("flaky", Some(500), 10, NOW),
            send("gone", Some(500), 10, NOW),
        ];
        let refs: Vec<&Send> = sends.iter().collect();
        let label = |name: &str| RequestLabel {
            name: name.to_owned(),
            method: "GET".to_owned(),
            kind: RequestKind::Http,
        };
        // "gone" was deleted, so it has no label and is left out.
        let labels = HashMap::from([("ok", label("Ok")), ("flaky", label("Flaky"))]);
        let endpoints = top_endpoints(&refs, window, &labels);
        let names: Vec<&str> = endpoints.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["Flaky", "Ok"]);
        assert_eq!(endpoints[0].error_rate, 50.0);
    }

    #[test]
    fn p95_uses_the_nearest_rank() {
        let mut times: Vec<i64> = (1..=20).collect();
        assert_eq!(p95(&mut times), Some(19));
        assert_eq!(p95(&mut [7]), Some(7));
        assert_eq!(p95(&mut []), None);
    }
}
