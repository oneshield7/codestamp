//! XML plumbing shared by the WSDL reader, the sample writer and the JSON converter.

use roxmltree::{Document, Node, ParsingOptions};
use url::Url;

use crate::error::AppError;
use crate::import::postman::MAX_IMPORT_BYTES;

pub const WSDL_NS: &str = "http://schemas.xmlsoap.org/wsdl/";
pub const WSDL2_NS: &str = "http://www.w3.org/ns/wsdl";
pub const SOAP11_BINDING_NS: &str = "http://schemas.xmlsoap.org/wsdl/soap/";
pub const SOAP12_BINDING_NS: &str = "http://schemas.xmlsoap.org/wsdl/soap12/";
pub const XML_NS: &str = "http://www.w3.org/XML/1998/namespace";
/// The XML Schema Recommendation, plus the drafts that older toolkits still write.
const XSD_NAMESPACES: [&str; 3] = [
    "http://www.w3.org/2001/XMLSchema",
    "http://www.w3.org/2000/10/XMLSchema",
    "http://www.w3.org/1999/XMLSchema",
];
/// Hosts that publish only standard schemas (XML Schema, SOAP encoding, xml.xsd, addressing):
/// their types count as built-in and their files are never downloaded.
const STANDARD_HOSTS: [&str; 2] = ["www.w3.org", "schemas.xmlsoap.org"];

/// A namespace-qualified name; `ns` is empty for a name in no namespace.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct QName<'a> {
    pub ns: &'a str,
    pub local: &'a str,
}

pub fn parse(text: &str) -> Result<Document<'_>, AppError> {
    if text.len() > MAX_IMPORT_BYTES {
        return Err(AppError::invalid("The XML is larger than 20 MB"));
    }
    // A DTD can declare entities that blow a small file up to gigabytes or pull in local
    // files, so a document that carries one is refused.
    let options = ParsingOptions {
        allow_dtd: false,
        ..ParsingOptions::default()
    };
    let text = text.trim_start_matches('\u{feff}').trim_start();
    Document::parse_with_options(text, options).map_err(|error| match error {
        roxmltree::Error::DtdDetected => AppError::invalid("XML with a DTD is not supported"),
        _ => AppError::invalid("The document is not valid XML"),
    })
}

pub fn is_in(node: Node, ns: &str, local: &str) -> bool {
    node.is_element() && node.tag_name().name() == local && node.tag_name().namespace() == Some(ns)
}

/// The local name of an XML Schema element, `None` for anything else.
pub fn xsd_name<'a>(node: Node<'a, '_>) -> Option<&'a str> {
    let name = node.tag_name();
    let in_xsd = node.is_element()
        && name
            .namespace()
            .is_some_and(|ns| XSD_NAMESPACES.contains(&ns));
    in_xsd.then(|| name.name())
}

pub fn is_xsd(node: Node, local: &str) -> bool {
    xsd_name(node) == Some(local)
}

pub fn children<'a, 'input>(
    node: Node<'a, 'input>,
    ns: &'static str,
    local: &'static str,
) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children()
        .filter(move |child| is_in(*child, ns, local))
}

pub fn xsd_children<'a, 'input>(
    node: Node<'a, 'input>,
    local: &'static str,
) -> impl Iterator<Item = Node<'a, 'input>> {
    node.children().filter(move |child| is_xsd(*child, local))
}

pub fn first_xsd_child<'a, 'input>(
    node: Node<'a, 'input>,
    locals: &[&str],
) -> Option<Node<'a, 'input>> {
    node.children()
        .find(|child| xsd_name(*child).is_some_and(|name| locals.contains(&name)))
}

/// Resolves a QName written in an attribute (`tns:Add`) with the prefixes in scope at `node`.
pub fn qname<'a>(node: Node<'a, '_>, value: &'a str) -> QName<'a> {
    let value = value.trim();
    let (prefix, local) = match value.split_once(':') {
        Some((prefix, local)) => (Some(prefix), local),
        None => (None, value),
    };
    // The parser does not list the `xml` prefix among those in scope, although the
    // Namespaces in XML spec binds it everywhere.
    let ns = match prefix {
        Some("xml") => XML_NS,
        _ => node.lookup_namespace_uri(prefix).unwrap_or_default(),
    };
    QName { ns, local }
}

pub fn is_standard_location(url: &Url) -> bool {
    url.host_str().is_some_and(|host| {
        STANDARD_HOSTS
            .iter()
            .any(|known| host.eq_ignore_ascii_case(known))
    })
}

/// Namespaces whose types are built in (XML Schema, SOAP encoding and the like): their values
/// are plain `?` and their schemas are not needed.
pub fn is_builtin(ns: &str) -> bool {
    Url::parse(ns).is_ok_and(|url| is_standard_location(&url))
}

pub fn is_ncname(name: &str) -> bool {
    let mut chars = name.chars();
    chars
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
        && chars.all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// An XML comment; text from the schema may hold "--", which a comment cannot.
pub fn comment(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 7);
    out.push_str("<!--");
    let mut last = ' ';
    for character in text.chars().filter(|c| !c.is_control()) {
        if character == '-' && last == '-' {
            out.push(' ');
        }
        out.push(character);
        last = character;
    }
    if last == '-' {
        out.push(' ');
    }
    out.push_str("-->");
    out
}

/// The namespace and local name of the root element, read from its start tag alone so a large
/// file is not parsed in full.
pub fn root_name(text: &str) -> Option<(String, String)> {
    let mut rest = text.trim_start_matches('\u{feff}');
    loop {
        rest = rest.trim_start();
        if let Some(after) = rest.strip_prefix("<?") {
            rest = after.split_once("?>")?.1;
        } else if let Some(after) = rest.strip_prefix("<!--") {
            rest = after.split_once("-->")?.1;
        } else if rest.starts_with('<') && !rest.starts_with("<!") {
            break;
        } else {
            return None;
        }
    }
    let tag = rest.get(..start_tag_end(rest)?)?;
    let snippet = format!("{}/>", tag.strip_suffix('/').unwrap_or(tag));
    let document = parse(&snippet).ok()?;
    let name = document.root_element().tag_name();
    Some((
        name.namespace().unwrap_or_default().to_owned(),
        name.name().to_owned(),
    ))
}

/// The element's name as written, prefix included; the parsed tree keeps only the namespace.
pub fn written_name<'t>(source: &'t str, node: Node) -> &'t str {
    let tag = source.get(node.range().start + 1..).unwrap_or_default();
    let end = tag
        .find(|c: char| c.is_whitespace() || c == '/' || c == '>')
        .unwrap_or(tag.len());
    tag.get(..end).unwrap_or_default()
}

/// Attribute names of the element's start tag as written, namespace declarations included,
/// which the parsed tree does not keep as attributes.
pub fn written_attribute_names<'t>(source: &'t str, node: Node) -> Vec<&'t str> {
    let tag = source.get(node.range().start..).unwrap_or_default();
    let tag = tag
        .get(..start_tag_end(tag).unwrap_or(tag.len()))
        .unwrap_or_default();
    let mut rest = tag
        .trim_start_matches('<')
        .trim_start_matches(|c: char| !c.is_whitespace() && c != '/');
    let mut names = Vec::new();
    loop {
        rest = rest.trim_start();
        if rest.is_empty() || rest.starts_with('/') {
            break;
        }
        let name_end = rest
            .find(|c: char| c == '=' || c.is_whitespace())
            .unwrap_or(rest.len());
        names.push(rest.get(..name_end).unwrap_or_default());
        let Some(open) = rest.find(['"', '\'']) else {
            break;
        };
        let quote = if rest.get(open..).is_some_and(|value| value.starts_with('"')) {
            '"'
        } else {
            '\''
        };
        let value = rest.get(open + 1..).unwrap_or_default();
        let Some(close) = value.find(quote) else {
            break;
        };
        rest = value.get(close + 1..).unwrap_or_default();
    }
    names
}

/// Where the start tag at the beginning of `tag` ends (its `>`), skipping quoted values.
fn start_tag_end(tag: &str) -> Option<usize> {
    let mut quote = None;
    for (index, byte) in tag.bytes().enumerate() {
        match (quote, byte) {
            (Some(open), _) if byte == open => quote = None,
            (Some(_), _) => {}
            (None, b'"' | b'\'') => quote = Some(byte),
            (None, b'>') => return Some(index),
            (None, _) => {}
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_root_name_from_the_start_tag() {
        let text = "\u{feff}<?xml version=\"1.0\"?>\n<!-- a > b -->\n<w:definitions a=\"x>y\" \
                    xmlns:w=\"http://schemas.xmlsoap.org/wsdl/\"><unclosed>";
        assert_eq!(
            root_name(text),
            Some((WSDL_NS.to_owned(), "definitions".to_owned()))
        );
        assert_eq!(root_name("<!DOCTYPE x><x/>"), None);
        assert_eq!(root_name("plain text"), None);
    }

    #[test]
    fn keeps_names_as_written() {
        let text = r#"<s:a xmlns:s="urn:s" s:x='1>' y = "2" xmlns="urn:d"/>"#;
        let document = parse(text).expect("xml");
        let root = document.root_element();
        assert_eq!(written_name(text, root), "s:a");
        assert_eq!(
            written_attribute_names(text, root),
            ["xmlns:s", "s:x", "y", "xmlns"]
        );
    }

    #[test]
    fn writes_safe_comments_and_attributes() {
        assert_eq!(comment("a--b-"), "<!--a- -b- -->");
        assert!(is_ncname("intA") && is_ncname("_x-1.y"));
        assert!(!is_ncname("1a") && !is_ncname("a b") && !is_ncname(""));
        assert!(is_builtin("http://www.w3.org/2001/XMLSchema"));
        assert!(!is_builtin("http://tempuri.org/"));
    }
}
