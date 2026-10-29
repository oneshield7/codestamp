//! XML to JSON the way Postman's `xml2Json` does it (xml2js with `explicitArray: false`,
//! `trim: true`, `mergeAttrs: false`), so test scripts written for Postman read SOAP
//! responses the same way.

use roxmltree::Node;
use serde_json::{Map, Value};

use super::xml::{self, written_attribute_names, written_name};
use crate::error::AppError;

/// Deeper documents are refused rather than risking the stack.
const MAX_DEPTH: usize = 256;
const ATTRIBUTES_KEY: &str = "$";
const TEXT_KEY: &str = "_";

pub fn xml_to_json(text: &str) -> Result<Value, AppError> {
    let document = xml::parse(text)?;
    let source = document.input_text();
    let root = document.root_element();
    let mut object = Map::new();
    object.insert(
        written_name(source, root).to_owned(),
        element(source, root, 0)?,
    );
    Ok(Value::Object(object))
}

/// A text-only element becomes its text; otherwise an object with attributes under `$`, text
/// under `_` and children by name, repeated names collected in an array.
fn element(source: &str, node: Node, depth: usize) -> Result<Value, AppError> {
    if depth > MAX_DEPTH {
        return Err(AppError::invalid("The XML is nested too deeply"));
    }
    let attributes = attributes(source, node);
    let mut children = Map::new();
    let mut text = String::new();
    for child in node.children() {
        if child.is_element() {
            let value = element(source, child, depth + 1)?;
            add_child(&mut children, written_name(source, child), value);
        } else if child.is_text() {
            text.push_str(child.text().unwrap_or_default());
        }
    }
    let text = text.trim();
    if attributes.is_empty() && children.is_empty() {
        return Ok(Value::String(text.to_owned()));
    }
    let mut object = Map::new();
    if !attributes.is_empty() {
        object.insert(ATTRIBUTES_KEY.to_owned(), Value::Object(attributes));
    }
    if !text.is_empty() {
        object.insert(TEXT_KEY.to_owned(), Value::String(text.to_owned()));
    }
    object.extend(children);
    Ok(Value::Object(object))
}

/// Attributes by the names written in the start tag; xml2js keeps namespace declarations
/// (`xmlns:soap`) among them.
fn attributes(source: &str, node: Node) -> Map<String, Value> {
    let mut attributes = Map::new();
    for name in written_attribute_names(source, node) {
        let value = if name == "xmlns" {
            node.lookup_namespace_uri(None)
        } else if let Some(prefix) = name.strip_prefix("xmlns:") {
            node.lookup_namespace_uri(Some(prefix))
        } else {
            node.attributes()
                .find(|attribute| source.get(attribute.range_qname()) == Some(name))
                .map(|attribute| attribute.value())
        };
        attributes.insert(
            name.to_owned(),
            Value::String(value.unwrap_or_default().to_owned()),
        );
    }
    attributes
}

fn add_child(children: &mut Map<String, Value>, name: &str, value: Value) {
    match children.get_mut(name) {
        Some(Value::Array(values)) => values.push(value),
        Some(existing) => {
            let first = existing.take();
            *existing = Value::Array(vec![first, value]);
        }
        None => {
            children.insert(name.to_owned(), value);
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn converts_like_postman_xml2json() {
        let value =
            xml_to_json(r#"<a x="1"><b>hi</b><b>yo</b><c/><d y="2">t</d></a>"#).expect("json");
        assert_eq!(
            value,
            json!({"a":{"$":{"x":"1"},"b":["hi","yo"],"c":"","d":{"$":{"y":"2"},"_":"t"}}})
        );
    }

    #[test]
    fn keeps_prefixes_and_namespace_declarations() {
        let value = xml_to_json(
            "<?xml version=\"1.0\"?>\n<soap:Envelope xmlns:soap=\"http://schemas.xmlsoap.org/soap/envelope/\">\
             <soap:Body><m:AddResponse xmlns:m=\"http://tempuri.org/\" xmlns=\"urn:d\">\
             <m:AddResult> 3 </m:AddResult></m:AddResponse></soap:Body></soap:Envelope>",
        )
        .expect("json");
        assert_eq!(
            value,
            json!({"soap:Envelope": {
                "$": {"xmlns:soap": "http://schemas.xmlsoap.org/soap/envelope/"},
                "soap:Body": {"m:AddResponse": {
                    "$": {"xmlns:m": "http://tempuri.org/", "xmlns": "urn:d"},
                    "m:AddResult": "3"
                }}
            }})
        );
    }

    #[test]
    fn handles_text_cdata_comments_and_mixed_content() {
        let value = xml_to_json(
            "<r><!-- note --><?pi x?><c><![CDATA[ <raw> ]]></c><m> before <i/> after </m>\
             <e a=\"&amp;\">  </e><w>  </w></r>",
        )
        .expect("json");
        assert_eq!(
            value,
            json!({"r": {
                "c": "<raw>",
                "m": {"_": "before  after", "i": ""},
                "e": {"$": {"a": "&"}},
                "w": ""
            }})
        );
    }

    #[test]
    fn refuses_dtds_and_oversized_input() {
        let error = xml_to_json("<!DOCTYPE r [<!ENTITY a \"aaaa\">]><r>&a;</r>").expect_err("dtd");
        assert_eq!(error.to_string(), "XML with a DTD is not supported");
        let huge = format!(
            "<r>{}</r>",
            " ".repeat(crate::import::postman::MAX_IMPORT_BYTES)
        );
        let error = xml_to_json(&huge).expect_err("too large");
        assert_eq!(error.to_string(), "The XML is larger than 20 MB");
        let deep = format!("{}{}", "<a>".repeat(300), "</a>".repeat(300));
        assert!(xml_to_json(&deep).is_err());
    }
}
