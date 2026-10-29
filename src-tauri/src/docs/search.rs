//! Docs in quick search: collections, sections and requests whose name or description holds
//! every word typed, with the part of the description where a word was found.

use rusqlite::{Connection, Row};

use super::models::DocHit;
use crate::error::AppError;
use crate::storage::descriptions::Described;
use crate::storage::icons;
use crate::storage::models::RequestKind;

/// Quick search shows only the first few, so the scan stops here.
const MAX_HITS: usize = 30;
const MAX_QUERY_CHARS: usize = 200;
const MAX_WORDS: usize = 8;
/// Characters kept before and after the first word found in a description.
const SNIPPET_BEFORE: usize = 40;
const SNIPPET_AFTER: usize = 100;

/// The lowercase words of a query; a blank query has none.
pub fn query_words(query: &str) -> Result<Vec<String>, AppError> {
    if query.chars().count() > MAX_QUERY_CHARS {
        return Err(AppError::invalid("A search can be up to 200 characters"));
    }
    let words: Vec<String> = query.split_whitespace().map(str::to_lowercase).collect();
    if words.len() > MAX_WORDS {
        return Err(AppError::invalid("A search can have up to 8 words"));
    }
    Ok(words)
}

/// Name matches come first, then description matches; each in notebook order.
pub fn search(conn: &Connection, words: &[String]) -> Result<Vec<DocHit>, AppError> {
    if words.is_empty() {
        return Ok(Vec::new());
    }
    let mut by_name = Vec::new();
    let mut by_description = Vec::new();
    let mut keep = |hit: Candidate| {
        if !holds_all(&format!("{}\n{}", hit.hit.name, hit.description), words) {
            return;
        }
        if holds_all(&hit.hit.name, words) {
            by_name.push(hit.hit);
        } else {
            let mut found = hit.hit;
            found.snippet = snippet(&hit.description, words);
            by_description.push(found);
        }
    };

    let mut collections =
        conn.prepare("SELECT id, name, icon, description FROM collections ORDER BY sort_order")?;
    for hit in collections.query_map([], collection_row)? {
        keep(hit?);
    }
    let mut sections = conn.prepare(
        "SELECT s.id, s.name, s.description, c.id, c.name, c.icon
         FROM sections s JOIN collections c ON c.id = s.collection_id
         ORDER BY c.sort_order, s.sort_order",
    )?;
    for hit in sections.query_map([], section_row)? {
        keep(hit?);
    }
    let mut requests = conn.prepare(
        "SELECT r.id, r.name, r.description, r.kind, r.method, c.id, c.name, c.icon, s.name
         FROM requests r
         JOIN collections c ON c.id = r.collection_id
         LEFT JOIN sections s ON s.id = r.section_id
         ORDER BY c.sort_order, COALESCE(s.sort_order, -1), r.sort_order",
    )?;
    for hit in requests.query_map([], request_row)? {
        keep(hit?);
    }

    by_name.extend(by_description);
    by_name.truncate(MAX_HITS);
    Ok(by_name)
}

/// A row read for the search, with the description it is matched on.
struct Candidate {
    hit: DocHit,
    description: String,
}

fn collection_row(row: &Row<'_>) -> rusqlite::Result<Candidate> {
    let name: String = row.get(1)?;
    let icon: Option<String> = row.get(2)?;
    Ok(Candidate {
        hit: DocHit {
            item: Described::Collection,
            id: row.get(0)?,
            collection_id: row.get(0)?,
            collection_icon: icons::resolve(&name, icon.as_deref()),
            collection_name: name.clone(),
            section_name: None,
            name,
            kind: None,
            method: None,
            snippet: None,
        },
        description: row.get(3)?,
    })
}

fn section_row(row: &Row<'_>) -> rusqlite::Result<Candidate> {
    let collection_name: String = row.get(4)?;
    let icon: Option<String> = row.get(5)?;
    Ok(Candidate {
        hit: DocHit {
            item: Described::Section,
            id: row.get(0)?,
            collection_id: row.get(3)?,
            collection_icon: icons::resolve(&collection_name, icon.as_deref()),
            collection_name,
            section_name: None,
            name: row.get(1)?,
            kind: None,
            method: None,
            snippet: None,
        },
        description: row.get(2)?,
    })
}

fn request_row(row: &Row<'_>) -> rusqlite::Result<Candidate> {
    let collection_name: String = row.get(6)?;
    let icon: Option<String> = row.get(7)?;
    Ok(Candidate {
        hit: DocHit {
            item: Described::Request,
            id: row.get(0)?,
            collection_id: row.get(5)?,
            collection_icon: icons::resolve(&collection_name, icon.as_deref()),
            collection_name,
            section_name: row.get(8)?,
            name: row.get(1)?,
            kind: Some(RequestKind::from_stored(&row.get::<_, String>(3)?)),
            method: Some(row.get(4)?),
            snippet: None,
        },
        description: row.get(2)?,
    })
}

fn holds_all(text: &str, words: &[String]) -> bool {
    let lower = text.to_lowercase();
    words.iter().all(|word| lower.contains(word.as_str()))
}

/// One line of the description around the first word found in it, Markdown marks left out.
fn snippet(description: &str, words: &[String]) -> Option<String> {
    let flat = description
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(['#', '*', '`'], "");
    let chars: Vec<char> = flat.chars().collect();
    let lower: Vec<char> = chars
        .iter()
        .map(|c| c.to_lowercase().next().unwrap_or(*c))
        .collect();
    let at = words
        .iter()
        .filter_map(|word| position(&lower, &word.chars().collect::<Vec<_>>()))
        .min()?;
    let start = at.saturating_sub(SNIPPET_BEFORE);
    let end = (at + SNIPPET_AFTER).min(chars.len());
    let mut text = String::new();
    if start > 0 {
        text.push('…');
    }
    text.extend(&chars[start..end]);
    if end < chars.len() {
        text.push('…');
    }
    Some(text.trim().to_owned())
}

fn position(haystack: &[char], needle: &[char]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::db::Database;
    use crate::storage::descriptions;
    use crate::storage::models::RequestContent;
    use crate::storage::repo;

    fn request(name: &str) -> RequestContent {
        serde_json::from_value(serde_json::json!({
            "name": name, "method": "GET", "url": "https://api.example.test/x",
            "params": [], "headers": [], "bodyMode": "none", "body": ""
        }))
        .expect("content")
    }

    #[test]
    fn checks_the_query() {
        assert_eq!(
            query_words("  Create   USER ").expect("words"),
            ["create", "user"]
        );
        assert!(query_words("").expect("blank").is_empty());
        assert!(query_words(&"a ".repeat(9)).is_err());
        assert!(query_words(&"x".repeat(201)).is_err());
    }

    #[test]
    fn finds_names_first_then_descriptions_with_a_snippet() {
        let db = Database::in_memory().expect("database");
        db.with(|conn| {
            let shop = repo::insert_collection(conn, "Shop")?;
            let orders = repo::insert_section(conn, &shop.id, "Orders")?;
            let refund = repo::insert_request(conn, &shop.id, Some(&orders.id), request("Refund"))?;
            let pay = repo::insert_request(conn, &shop.id, None, request("Pay invoice"))?;
            descriptions::set(
                conn,
                Described::Request,
                &refund.id,
                "## Refund\n\nReturns the **money** of an invoice to the buyer.",
            )?;
            descriptions::set(
                conn,
                Described::Section,
                &orders.id,
                "Everything after checkout",
            )?;

            let hits = search(conn, &query_words("invoice")?)?;
            let names: Vec<&str> = hits.iter().map(|hit| hit.name.as_str()).collect();
            assert_eq!(names, ["Pay invoice", "Refund"]);
            assert_eq!(hits[0].id, pay.id);
            assert_eq!(hits[0].snippet, None);
            assert_eq!(hits[1].section_name.as_deref(), Some("Orders"));
            assert_eq!(
                hits[1].snippet.as_deref(),
                Some("Refund Returns the money of an invoice to the buyer.")
            );

            let words = query_words("CHECKOUT everything")?;
            let section = search(conn, &words)?;
            assert_eq!(section.len(), 1);
            assert_eq!(section[0].item, Described::Section);
            assert!(search(conn, &query_words("invoice missing")?)?.is_empty());
            Ok(())
        })
        .expect("search");
    }

    #[test]
    fn cuts_long_descriptions_around_the_word() {
        let long = format!("{} needle {}", "a".repeat(100), "b".repeat(200));
        let text = snippet(&long, &["needle".to_owned()]).expect("snippet");
        assert!(text.starts_with('…') && text.ends_with('…'));
        assert!(text.contains("needle"));
        assert!(text.chars().count() <= SNIPPET_BEFORE + SNIPPET_AFTER + 2);
        assert_eq!(snippet("no match here", &["zzz".to_owned()]), None);
    }
}
