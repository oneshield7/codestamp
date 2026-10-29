use url::form_urlencoded;

use crate::storage::models::KeyValue;

/// Splits `base?query#fragment` into the base URL and decoded query rows. Works on templates
/// such as `{{baseUrl}}/users?page={{page}}`, which are not valid URLs yet. Values are
/// decoded because the sender encodes parameters again when it builds the final URL.
pub fn split_url(raw: &str) -> (String, Vec<KeyValue>) {
    let without_fragment = raw.split('#').next().unwrap_or_default().trim();
    match without_fragment.split_once('?') {
        Some((base, query)) => (base.to_owned(), parse_query(query)),
        None => (without_fragment.to_owned(), Vec::new()),
    }
}

pub fn parse_query(query: &str) -> Vec<KeyValue> {
    form_urlencoded::parse(query.as_bytes())
        .filter(|(key, _)| !key.is_empty())
        .map(|(key, value)| KeyValue {
            enabled: true,
            key: key.into_owned(),
            value: value.into_owned(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_templates_and_decodes_values() {
        let (base, params) = split_url("{{baseUrl}}/users?page={{page}}&q=rina%20budi#top");
        assert_eq!(base, "{{baseUrl}}/users");
        let pairs: Vec<_> = params
            .iter()
            .map(|p| (p.key.as_str(), p.value.as_str()))
            .collect();
        assert_eq!(pairs, [("page", "{{page}}"), ("q", "rina budi")]);
    }

    #[test]
    fn leaves_urls_without_query_alone() {
        assert_eq!(split_url("https://api.test/v1").0, "https://api.test/v1");
        assert!(split_url("https://api.test/v1").1.is_empty());
    }
}
