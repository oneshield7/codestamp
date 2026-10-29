//! Three-way merge of the repository's JSON files, field by field. Changes made on only one side
//! are taken as they are; a field both sides changed differently is a conflict for the user,
//! unless a choice for that file was already made.

use serde_json::{Map, Value};

use super::files::{COLLECTION, environment_id_of, request_id_of};
use super::models::Choice;
use super::present::label;

/// A field both sides changed. Labels are shown as they are in the conflict dialog.
#[derive(Debug, Clone, PartialEq)]
pub struct FieldConflict {
    pub label: String,
    pub mine: Option<Value>,
    pub theirs: Option<Value>,
}

#[derive(Debug, Default)]
pub struct Merged {
    /// `None` when the file is deleted.
    pub value: Option<Value>,
    pub conflicts: Vec<FieldConflict>,
    /// With "keep both", the full local version, to be saved as a copy.
    pub copy: Option<Value>,
}

impl Merged {
    fn clean(value: Option<&Value>) -> Self {
        Self {
            value: value.cloned(),
            ..Self::default()
        }
    }
}

pub fn merge_file(
    path: &str,
    base: Option<&Value>,
    mine: Option<&Value>,
    theirs: Option<&Value>,
    choice: Option<Choice>,
) -> Merged {
    if mine == theirs || base == theirs {
        return Merged::clean(mine);
    }
    if base == mine {
        return Merged::clean(theirs);
    }
    match (mine, theirs) {
        // An edit wins over a delete: work is never thrown away silently.
        (None, edited) | (edited, None) => Merged::clean(edited),
        (Some(mine), Some(theirs)) => {
            let base_map = base.and_then(Value::as_object);
            match (mine.as_object(), theirs.as_object()) {
                (Some(mine_map), Some(theirs_map)) if path == COLLECTION => {
                    merge_collection(base_map, mine_map, theirs_map, choice)
                }
                (Some(mine_map), Some(theirs_map)) if request_id_of(path).is_some() => {
                    merge_request(base_map, mine_map, theirs_map, mine, choice)
                }
                (Some(mine_map), Some(theirs_map)) if environment_id_of(path).is_some() => {
                    merge_environment(base_map, mine_map, theirs_map, choice)
                }
                _ => {
                    let mut conflicts = Vec::new();
                    let value = field(
                        "File",
                        base,
                        Some(mine),
                        Some(theirs),
                        choice,
                        &mut conflicts,
                    );
                    Merged {
                        value,
                        conflicts,
                        copy: None,
                    }
                }
            }
        }
    }
}

fn merge_request(
    base: Option<&Map<String, Value>>,
    mine: &Map<String, Value>,
    theirs: &Map<String, Value>,
    mine_value: &Value,
    choice: Option<Choice>,
) -> Merged {
    let mut conflicts = Vec::new();
    let mut merged = Map::new();
    for key in keys(mine, theirs) {
        if let Some(value) = field(
            label(&key),
            base.and_then(|b| b.get(&key)),
            mine.get(&key),
            theirs.get(&key),
            choice,
            &mut conflicts,
        ) {
            merged.insert(key, value);
        }
    }
    let kept_both = choice == Some(Choice::Both) && had_conflict(base, mine, theirs);
    Merged {
        value: Some(Value::Object(merged)),
        conflicts,
        copy: kept_both.then(|| mine_value.clone()),
    }
}

/// Whether any field was changed differently on both sides (for "keep both").
fn had_conflict(
    base: Option<&Map<String, Value>>,
    mine: &Map<String, Value>,
    theirs: &Map<String, Value>,
) -> bool {
    keys(mine, theirs).iter().any(|key| {
        let (b, m, t) = (
            base.and_then(|b| b.get(key)),
            mine.get(key),
            theirs.get(key),
        );
        m != t && b != m && b != t && m.is_some() && t.is_some()
    })
}

fn merge_collection(
    base: Option<&Map<String, Value>>,
    mine: &Map<String, Value>,
    theirs: &Map<String, Value>,
    choice: Option<Choice>,
) -> Merged {
    // "Keep both" makes no sense for the collection itself; it falls back to theirs.
    let choice = choice.map(|c| if c == Choice::Both { Choice::Theirs } else { c });
    let mut conflicts = Vec::new();
    let mut merged = Map::new();
    for key in keys(mine, theirs) {
        let (b, m, t) = (
            base.and_then(|b| b.get(&key)),
            mine.get(&key),
            theirs.get(&key),
        );
        let value = match key.as_str() {
            "sections" => Some(merge_list(b, m, t, "id", "Section", choice, &mut conflicts)),
            "variables" => Some(merge_list(
                b,
                m,
                t,
                "key",
                "Variable",
                choice,
                &mut conflicts,
            )),
            "order" => Some(merge_order(b, m, t)),
            "name" => field("Collection name", b, m, t, choice, &mut conflicts),
            "auth" => field("Collection auth", b, m, t, choice, &mut conflicts),
            "icon" => field("Collection icon", b, m, t, choice, &mut conflicts),
            other => field(other, b, m, t, choice, &mut conflicts),
        };
        if let Some(value) = value {
            merged.insert(key, value);
        }
    }
    Merged {
        value: Some(Value::Object(merged)),
        conflicts,
        copy: None,
    }
}

/// Name and variables (by name) of a shared environment, like the collection's own.
fn merge_environment(
    base: Option<&Map<String, Value>>,
    mine: &Map<String, Value>,
    theirs: &Map<String, Value>,
    choice: Option<Choice>,
) -> Merged {
    let choice = choice.map(|c| if c == Choice::Both { Choice::Theirs } else { c });
    let mut conflicts = Vec::new();
    let mut merged = Map::new();
    for key in keys(mine, theirs) {
        let (b, m, t) = (
            base.and_then(|b| b.get(&key)),
            mine.get(&key),
            theirs.get(&key),
        );
        let value = match key.as_str() {
            "variables" => Some(merge_list(
                b,
                m,
                t,
                "key",
                "Variable",
                choice,
                &mut conflicts,
            )),
            "name" => field("Environment name", b, m, t, choice, &mut conflicts),
            other => field(other, b, m, t, choice, &mut conflicts),
        };
        if let Some(value) = value {
            merged.insert(key, value);
        }
    }
    Merged {
        value: Some(Value::Object(merged)),
        conflicts,
        copy: None,
    }
}

/// Merges one value. A side that removed it loses to a side that changed it.
fn field(
    label: &str,
    base: Option<&Value>,
    mine: Option<&Value>,
    theirs: Option<&Value>,
    choice: Option<Choice>,
    conflicts: &mut Vec<FieldConflict>,
) -> Option<Value> {
    if mine == theirs || base == theirs {
        return mine.cloned();
    }
    if base == mine {
        return theirs.cloned();
    }
    match (mine, theirs) {
        (None, edited) | (edited, None) => edited.cloned(),
        _ => match choice {
            Some(Choice::Mine) => mine.cloned(),
            Some(Choice::Theirs | Choice::Both) => theirs.cloned(),
            None => {
                conflicts.push(FieldConflict {
                    label: label.to_owned(),
                    mine: mine.cloned(),
                    theirs: theirs.cloned(),
                });
                mine.cloned()
            }
        },
    }
}

/// Lists of objects identified by `key` (sections by id, variables by name), merged item by
/// item. The result follows the remote order, with items added here appended.
fn merge_list(
    base: Option<&Value>,
    mine: Option<&Value>,
    theirs: Option<&Value>,
    key: &str,
    label: &str,
    choice: Option<Choice>,
    conflicts: &mut Vec<FieldConflict>,
) -> Value {
    let items = |list: Option<&Value>| -> Vec<(String, Value)> {
        list.and_then(Value::as_array)
            .map(|array| {
                array
                    .iter()
                    .filter_map(|item| Some((item.get(key)?.as_str()?.to_owned(), item.clone())))
                    .collect()
            })
            .unwrap_or_default()
    };
    let (base, mine, theirs) = (items(base), items(mine), items(theirs));
    let find = |list: &[(String, Value)], id: &str| {
        list.iter()
            .find(|(own, _)| own == id)
            .map(|(_, v)| v.clone())
    };
    let mut ids: Vec<String> = theirs.iter().map(|(id, _)| id.clone()).collect();
    for (id, _) in &mine {
        if !ids.contains(id) {
            ids.push(id.clone());
        }
    }
    let mut merged = Vec::new();
    for id in ids {
        let (b, m, t) = (find(&base, &id), find(&mine, &id), find(&theirs, &id));
        let name = m
            .as_ref()
            .or(t.as_ref())
            .and_then(|item| item.get("name").or_else(|| item.get(key)))
            .and_then(Value::as_str)
            .unwrap_or(&id)
            .to_owned();
        if let Some(value) = field(
            &format!("{label} “{name}”"),
            b.as_ref(),
            m.as_ref(),
            t.as_ref(),
            choice,
            conflicts,
        ) {
            merged.push(value);
        }
    }
    Value::Array(merged)
}

/// Request order: if only one side reordered, that order wins. If both did, the remote order is
/// kept, without the ids removed here and with the ids added here appended.
fn merge_order(base: Option<&Value>, mine: Option<&Value>, theirs: Option<&Value>) -> Value {
    let ids = |list: Option<&Value>| -> Vec<String> {
        list.and_then(Value::as_array)
            .map(|array| {
                array
                    .iter()
                    .filter_map(|id| id.as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default()
    };
    let (base, mine, theirs) = (ids(base), ids(mine), ids(theirs));
    let chosen = if mine == theirs || base == theirs {
        mine
    } else if base == mine {
        theirs
    } else {
        let removed_here = |id: &String| base.contains(id) && !mine.contains(id);
        let mut order: Vec<String> = theirs.into_iter().filter(|id| !removed_here(id)).collect();
        for id in mine {
            if !base.contains(&id) && !order.contains(&id) {
                order.push(id);
            }
        }
        order
    };
    Value::Array(chosen.into_iter().map(Value::String).collect())
}

/// Keys of both objects, in the local order, then keys only the remote has.
fn keys(mine: &Map<String, Value>, theirs: &Map<String, Value>) -> Vec<String> {
    let mut keys: Vec<String> = mine.keys().cloned().collect();
    for key in theirs.keys() {
        if !mine.contains_key(key) {
            keys.push(key.clone());
        }
    }
    keys
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    const REQUEST: &str = "requests/8f2c1a7e-0000-4000-8000-000000000000.json";

    fn request(url: &str, headers: Value, script: &str) -> Value {
        json!({ "id": "8f2c1a7e-0000-4000-8000-000000000000", "name": "List users", "url": url, "headers": headers, "testScript": script })
    }

    #[test]
    fn takes_changes_made_on_one_side() {
        let base = request("/a", json!([]), "");
        let mine = request("/a", json!([]), "pm.test('x')");
        let merged = merge_file(REQUEST, Some(&base), Some(&mine), Some(&base), None);
        assert_eq!(merged.value, Some(mine.clone()));
        let merged = merge_file(REQUEST, Some(&base), Some(&base), Some(&mine), None);
        assert_eq!(merged.value, Some(mine));
    }

    #[test]
    fn merges_different_fields_of_the_same_request() {
        let base = request("/a", json!([]), "");
        let mine = request("/b", json!([]), "");
        let theirs = request(
            "/a",
            json!([{ "enabled": true, "key": "X-Team", "value": "1" }]),
            "",
        );
        let merged = merge_file(REQUEST, Some(&base), Some(&mine), Some(&theirs), None);
        assert!(merged.conflicts.is_empty());
        let value = merged.value.expect("merged");
        assert_eq!(value["url"], "/b");
        assert_eq!(value["headers"][0]["key"], "X-Team");
    }

    #[test]
    fn reports_the_same_field_changed_differently_and_applies_choices() {
        let base = request("/a", json!([]), "");
        let mine = request("/mine", json!([]), "");
        let theirs = request("/theirs", json!([]), "pm.test('t')");
        let merged = merge_file(REQUEST, Some(&base), Some(&mine), Some(&theirs), None);
        assert_eq!(merged.conflicts.len(), 1);
        assert_eq!(merged.conflicts[0].label, "URL");

        let theirs_wins = merge_file(
            REQUEST,
            Some(&base),
            Some(&mine),
            Some(&theirs),
            Some(Choice::Theirs),
        );
        assert!(theirs_wins.conflicts.is_empty());
        assert_eq!(
            theirs_wins.value.as_ref().map(|v| v["url"].clone()),
            Some(json!("/theirs"))
        );

        let mine_wins = merge_file(
            REQUEST,
            Some(&base),
            Some(&mine),
            Some(&theirs),
            Some(Choice::Mine),
        );
        let value = mine_wins.value.expect("merged");
        assert_eq!(value["url"], "/mine");
        assert_eq!(
            value["testScript"], "pm.test('t')",
            "their other change is kept"
        );

        let both = merge_file(
            REQUEST,
            Some(&base),
            Some(&mine),
            Some(&theirs),
            Some(Choice::Both),
        );
        assert_eq!(
            both.value.as_ref().map(|v| v["url"].clone()),
            Some(json!("/theirs"))
        );
        assert_eq!(both.copy.map(|v| v["url"].clone()), Some(json!("/mine")));
    }

    #[test]
    fn an_edit_wins_over_a_delete() {
        let base = request("/a", json!([]), "");
        let edited = request("/b", json!([]), "");
        assert_eq!(
            merge_file(REQUEST, Some(&base), None, Some(&edited), None).value,
            Some(edited.clone())
        );
        assert_eq!(
            merge_file(REQUEST, Some(&base), Some(&edited), None, None).value,
            Some(edited)
        );
        assert_eq!(
            merge_file(REQUEST, Some(&base), None, Some(&base), None).value,
            None
        );
    }

    #[test]
    fn merges_collection_lists_and_order() {
        let base = json!({
            "name": "API", "order": ["a", "b", "c"],
            "sections": [{ "id": "s1", "name": "Users" }],
            "variables": [{ "key": "baseUrl", "value": "1" }, { "key": "old", "value": "x" }]
        });
        let mine = json!({
            "name": "API", "order": ["c", "a", "b", "mine"],
            "sections": [{ "id": "s1", "name": "Users" }, { "id": "s2", "name": "Mine" }],
            "variables": [{ "key": "baseUrl", "value": "2" }]
        });
        let theirs = json!({
            "name": "API v2", "order": ["b", "a", "c", "theirs"],
            "sections": [{ "id": "s1", "name": "People" }],
            "variables": [{ "key": "baseUrl", "value": "1" }, { "key": "old", "value": "x" }, { "key": "new", "value": "y" }]
        });
        let merged = merge_file(COLLECTION, Some(&base), Some(&mine), Some(&theirs), None);
        assert!(merged.conflicts.is_empty(), "{:?}", merged.conflicts);
        let value = merged.value.expect("merged");
        assert_eq!(value["name"], "API v2");
        assert_eq!(value["order"], json!(["b", "a", "c", "theirs", "mine"]));
        assert_eq!(
            value["sections"],
            json!([{ "id": "s1", "name": "People" }, { "id": "s2", "name": "Mine" }])
        );
        assert_eq!(
            value["variables"],
            json!([{ "key": "baseUrl", "value": "2" }, { "key": "new", "value": "y" }])
        );
    }

    #[test]
    fn reports_a_variable_changed_on_both_sides() {
        let base = json!({ "variables": [{ "key": "baseUrl", "value": "1" }] });
        let mine = json!({ "variables": [{ "key": "baseUrl", "value": "2" }] });
        let theirs = json!({ "variables": [{ "key": "baseUrl", "value": "3" }] });
        let merged = merge_file(COLLECTION, Some(&base), Some(&mine), Some(&theirs), None);
        assert_eq!(merged.conflicts.len(), 1);
        assert_eq!(merged.conflicts[0].label, "Variable “baseUrl”");
    }

    #[test]
    fn merges_the_collection_icon_like_the_name() {
        let base = json!({ "name": "API" });
        let mine = json!({ "name": "API", "icon": "🦊" });
        let taken = merge_file(COLLECTION, Some(&base), Some(&mine), Some(&base), None);
        assert_eq!(taken.value.expect("value")["icon"], "🦊");

        let theirs = json!({ "name": "API", "icon": "🚀" });
        let both = merge_file(COLLECTION, Some(&base), Some(&mine), Some(&theirs), None);
        assert_eq!(both.conflicts.len(), 1);
        assert_eq!(both.conflicts[0].label, "Collection icon");
        let decided = merge_file(
            COLLECTION,
            Some(&base),
            Some(&mine),
            Some(&theirs),
            Some(Choice::Theirs),
        );
        assert_eq!(decided.value.expect("value")["icon"], "🚀");
    }
}
