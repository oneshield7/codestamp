//! Sample request envelopes in SoapUI's style: `?` for every value, comments for optional,
//! repeated and alternative content, and short namespace prefixes declared on the Envelope.

use roxmltree::Node;

use super::model::{SoapVersion, Warnings};
use super::schema::{MaxOccurs, Schemas, is_qualified, occurs, target_namespace};
use super::xml::{
    QName, XML_NS, comment, first_xsd_child, is_builtin, is_ncname, is_xsd, qname, xsd_children,
    xsd_name,
};
use crate::text::{clip, escape_xml};

const SOAP11_ENVELOPE: &str = "http://schemas.xmlsoap.org/soap/envelope/";
const SOAP12_ENVELOPE: &str = "http://www.w3.org/2003/05/soap-envelope";
const INDENT: &str = "  ";
/// Complex elements nested deeper than this are left out, so recursive schemas end.
const MAX_DEPTH: usize = 12;
/// How many times one type may be open inside itself before the sample stops expanding it.
const MAX_TYPE_REPEATS: usize = 2;
/// Model groups (sequence, choice, group references) nested inside one element.
const MAX_GROUP_NESTING: usize = 32;
/// Base types and attribute groups followed from one type.
const MAX_HOPS: usize = 16;
/// Schema nodes one envelope may visit, so a schema whose groups fan out cannot keep the
/// writer busy without writing anything.
const MAX_STEPS: usize = 100_000;
pub const MAX_ENVELOPE_BYTES: usize = 256 * 1024;
const MAX_ENUMERATION_TEXT: usize = 200;
/// Prefixes that readers associate with well-known namespaces.
const RESERVED_PREFIXES: [&str; 3] = ["xs", "xsd", "xsi"];
/// Namespace path segments too generic to name a prefix after.
const GENERIC_SEGMENTS: [&str; 8] = [
    "xsd", "wsdl", "schema", "schemas", "types", "type", "ns", "soap",
];
const PARTICLES: [&str; 5] = ["element", "sequence", "choice", "group", "any"];
const MODEL_GROUPS: [&str; 4] = ["sequence", "choice", "all", "group"];

/// What goes into the Header or Body: a global element, or an element named after a message
/// part that gives a type instead (rpc, or document style that skips elements).
#[derive(Debug, Clone, Copy)]
pub enum Payload<'a> {
    Element(QName<'a>),
    Typed {
        name: &'a str,
        type_name: Option<QName<'a>>,
    },
}

/// The rpc-style wrapper element, named after the operation.
#[derive(Debug)]
pub struct Wrapper<'a> {
    pub name: &'a str,
    pub namespace: &'a str,
    pub encoding_style: Option<&'a str>,
}

#[derive(Debug)]
pub struct Request<'a> {
    pub version: SoapVersion,
    pub headers: Vec<Payload<'a>>,
    pub body: Vec<Payload<'a>>,
    pub wrapper: Option<Wrapper<'a>>,
}

pub fn default_encoding_style(version: SoapVersion) -> &'static str {
    match version {
        SoapVersion::V11 => "http://schemas.xmlsoap.org/soap/encoding/",
        SoapVersion::V12 => "http://www.w3.org/2003/05/soap-encoding",
    }
}

/// Writes the sample for one operation; `limit` caps its size in bytes.
pub fn envelope<'a>(
    schemas: &Schemas<'a, '_>,
    request: &Request<'a>,
    operation: &str,
    limit: usize,
    warnings: &mut Warnings,
) -> String {
    let (env, env_ns) = match request.version {
        SoapVersion::V11 => ("soapenv", SOAP11_ENVELOPE),
        SoapVersion::V12 => ("soap", SOAP12_ENVELOPE),
    };
    let mut writer = Writer {
        schemas,
        warnings,
        env,
        prefixes: Prefixes(vec![(env.to_owned(), env_ns.to_owned())]),
        open_types: Vec::new(),
        written: 0,
        steps: 0,
        limit,
        cut: false,
    };
    let mut header = String::new();
    for payload in &request.headers {
        writer.payload(&mut header, 2, *payload);
    }
    let mut body = String::new();
    match &request.wrapper {
        Some(wrapper) => writer.wrapper(&mut body, 2, wrapper, &request.body),
        None => {
            for payload in &request.body {
                writer.payload(&mut body, 2, *payload);
            }
        }
    }
    if writer.cut {
        writer
            .warnings
            .add(format!("The sample for {operation} was cut short"));
    }

    let mut out = format!("<{env}:Envelope");
    for (prefix, ns) in &writer.prefixes.0 {
        out.push_str(&format!(" xmlns:{prefix}=\"{}\"", escape_xml(ns)));
    }
    out.push_str(">\n");
    section(&mut out, env, "Header", &header);
    section(&mut out, env, "Body", &body);
    out.push_str(&format!("</{env}:Envelope>"));
    out
}

fn section(out: &mut String, env: &str, name: &str, content: &str) {
    if content.is_empty() {
        out.push_str(&format!("{INDENT}<{env}:{name}/>\n"));
    } else {
        out.push_str(&format!(
            "{INDENT}<{env}:{name}>\n{content}{INDENT}</{env}:{name}>\n"
        ));
    }
}

/// What an element's type gives it.
enum TypeRef<'a, 'input> {
    Named(QName<'a>),
    Anonymous(Node<'a, 'input>),
    /// No type at all, which XML Schema reads as anyType.
    Any,
}

enum Resolved<'a, 'input> {
    Simple { enumeration: Option<String> },
    Complex(Node<'a, 'input>),
}

enum Content<'a, 'input> {
    Empty,
    Text,
    Particles(Vec<Node<'a, 'input>>),
}

struct Writer<'s, 'a, 'input> {
    schemas: &'s Schemas<'a, 'input>,
    warnings: &'s mut Warnings,
    env: &'static str,
    prefixes: Prefixes,
    /// Complex types being written, outermost first.
    open_types: Vec<Node<'a, 'input>>,
    written: usize,
    steps: usize,
    limit: usize,
    cut: bool,
}

impl<'a, 'input> Writer<'_, 'a, 'input> {
    fn line(&mut self, out: &mut String, depth: usize, text: &str) {
        for _ in 0..depth {
            out.push_str(INDENT);
        }
        out.push_str(text);
        out.push('\n');
        self.written += depth * INDENT.len() + text.len() + 1;
    }

    /// False once the sample is too large or too much work; the first time, a comment marks
    /// the spot.
    fn room(&mut self, out: &mut String, depth: usize) -> bool {
        if self.cut {
            return false;
        }
        self.steps += 1;
        if self.written < self.limit && self.steps <= MAX_STEPS {
            return true;
        }
        self.cut = true;
        self.line(
            out,
            depth,
            &comment("The sample stops here: it is too large"),
        );
        false
    }

    fn payload(&mut self, out: &mut String, depth: usize, payload: Payload<'a>) {
        match payload {
            Payload::Element(name) => {
                let type_ref = self.global_element_type(name);
                self.element(out, depth, name, type_ref, None);
            }
            Payload::Typed { name, type_name } => {
                let type_ref = type_name.map_or(TypeRef::Any, TypeRef::Named);
                self.element(
                    out,
                    depth,
                    QName {
                        ns: "",
                        local: name,
                    },
                    type_ref,
                    None,
                );
            }
        }
    }

    fn wrapper(
        &mut self,
        out: &mut String,
        depth: usize,
        wrapper: &Wrapper<'a>,
        parts: &[Payload<'a>],
    ) {
        if !is_ncname(wrapper.name) {
            return;
        }
        let tag = self.tag(QName {
            ns: wrapper.namespace,
            local: wrapper.name,
        });
        let style = wrapper.encoding_style.map_or_else(String::new, |style| {
            format!(" {}:encodingStyle=\"{}\"", self.env, escape_xml(style))
        });
        let mut inner = String::new();
        for part in parts {
            self.payload(&mut inner, depth + 1, *part);
        }
        self.enclose(out, depth, &format!("<{tag}{style}"), &tag, &inner);
    }

    /// Writes `start` as an empty element, or around `inner` when there is content.
    fn enclose(&mut self, out: &mut String, depth: usize, start: &str, tag: &str, inner: &str) {
        if inner.is_empty() {
            self.line(out, depth, &format!("{start}/>"));
        } else {
            self.line(out, depth, &format!("{start}>"));
            out.push_str(inner);
            self.line(out, depth, &format!("</{tag}>"));
        }
    }

    /// One element; `name.ns` is empty for an unqualified element.
    fn element(
        &mut self,
        out: &mut String,
        depth: usize,
        name: QName<'a>,
        type_ref: TypeRef<'a, 'input>,
        occurrence: Option<String>,
    ) {
        if !is_ncname(name.local) || !self.room(out, depth) {
            return;
        }
        let resolved = self.resolve(type_ref);
        let too_deep = match resolved {
            Resolved::Complex(node) => {
                self.open_types.len() >= MAX_DEPTH
                    || self.open_types.iter().filter(|open| **open == node).count()
                        >= MAX_TYPE_REPEATS
            }
            Resolved::Simple { .. } => false,
        };
        if too_deep {
            let text = format!("{} is left out: nested too deep", name.local);
            self.line(out, depth, &comment(&text));
            return;
        }
        let tag = self.tag(name);
        if let Some(text) = occurrence {
            self.line(out, depth, &comment(&text));
        }
        match resolved {
            Resolved::Simple { enumeration } => {
                if let Some(text) = enumeration {
                    self.line(out, depth, &comment(&text));
                }
                self.line(out, depth, &format!("<{tag}>?</{tag}>"));
            }
            Resolved::Complex(node) => self.complex(out, depth, &tag, node),
        }
    }

    fn complex(&mut self, out: &mut String, depth: usize, tag: &str, type_node: Node<'a, 'input>) {
        let mut attributes = Vec::new();
        self.type_attributes(type_node, &mut attributes, 0);
        let mut start = format!("<{tag}");
        for attribute in attributes {
            if is_ncname(attribute.local) {
                let name = self.tag(attribute);
                start.push_str(&format!(" {name}=\"?\""));
            }
        }
        match self.content(type_node, 0) {
            Content::Empty => self.line(out, depth, &format!("{start}/>")),
            Content::Text => self.line(out, depth, &format!("{start}>?</{tag}>")),
            Content::Particles(particles) => {
                self.open_types.push(type_node);
                let mut inner = String::new();
                for particle in particles {
                    self.particle(&mut inner, depth + 1, particle, 0);
                }
                self.open_types.pop();
                self.enclose(out, depth, &start, tag, &inner);
            }
        }
    }

    fn particle(&mut self, out: &mut String, depth: usize, node: Node<'a, 'input>, nesting: usize) {
        if nesting > MAX_GROUP_NESTING || !self.room(out, depth) {
            return;
        }
        match xsd_name(node) {
            Some("element") => self.local_element(out, depth, node),
            Some("sequence" | "all") => {
                for child in node.children().filter(Node::is_element) {
                    self.particle(out, depth, child, nesting + 1);
                }
            }
            Some("choice") => {
                let options: Vec<Node> = node
                    .children()
                    .filter(|child| xsd_name(*child).is_some_and(|name| PARTICLES.contains(&name)))
                    .collect();
                if options.is_empty() {
                    return;
                }
                let text = format!(
                    "You have a CHOICE of the next {} items at this level",
                    options.len()
                );
                self.line(out, depth, &comment(&text));
                for option in options {
                    self.particle(out, depth, option, nesting + 1);
                }
            }
            Some("group") => {
                let group = node
                    .attribute("ref")
                    .and_then(|reference| self.schemas.group(qname(node, reference)));
                if let Some(inner) = group.and_then(|group| first_xsd_child(group, &MODEL_GROUPS)) {
                    self.particle(out, depth, inner, nesting + 1);
                }
            }
            Some("any") => {
                self.line(
                    out,
                    depth,
                    &comment("You may enter ANY elements at this point"),
                );
            }
            _ => {}
        }
    }

    fn local_element(&mut self, out: &mut String, depth: usize, node: Node<'a, 'input>) {
        let (min, max) = occurs(node);
        if max == MaxOccurs::Bounded(0) {
            return;
        }
        let occurrence = occurrence_comment(min, max);
        if let Some(reference) = node.attribute("ref") {
            let name = qname(node, reference);
            let type_ref = self.global_element_type(name);
            self.element(out, depth, name, type_ref, occurrence);
            return;
        }
        let Some(local) = node.attribute("name") else {
            return;
        };
        let ns = if is_qualified(node, "elementFormDefault") {
            target_namespace(node)
        } else {
            ""
        };
        self.element(out, depth, QName { ns, local }, type_of(node), occurrence);
    }

    fn resolve(&mut self, type_ref: TypeRef<'a, 'input>) -> Resolved<'a, 'input> {
        match type_ref {
            TypeRef::Any => Resolved::Simple { enumeration: None },
            TypeRef::Anonymous(node) if is_xsd(node, "simpleType") => Resolved::Simple {
                enumeration: self.enumeration(node, None),
            },
            TypeRef::Anonymous(node) => Resolved::Complex(node),
            TypeRef::Named(name) if is_builtin(name.ns) => Resolved::Simple { enumeration: None },
            TypeRef::Named(name) => {
                if let Some(node) = self.schemas.complex_type(name) {
                    return Resolved::Complex(node);
                }
                let enumeration = match self.schemas.simple_type(name) {
                    Some(node) => self.enumeration(node, Some(name.local)),
                    None => {
                        self.warnings.add(format!(
                            "The type {} is not defined; its values show ?",
                            name.local
                        ));
                        None
                    }
                };
                Resolved::Simple { enumeration }
            }
        }
    }

    /// The type of a global element, with a warning when the element is not defined.
    fn global_element_type(&mut self, name: QName<'a>) -> TypeRef<'a, 'input> {
        if let Some(declaration) = self.schemas.element(name) {
            return type_of(declaration);
        }
        if !is_builtin(name.ns) {
            self.warnings.add(format!(
                "The element {} is not defined; its sample shows ?",
                name.local
            ));
        }
        TypeRef::Any
    }

    /// SoapUI's type comment for an enumeration, following restrictions of other simple types.
    fn enumeration(&self, simple_type: Node<'a, 'input>, name: Option<&str>) -> Option<String> {
        let mut node = simple_type;
        for _ in 0..MAX_HOPS {
            let restriction = first_xsd_child(node, &["restriction"])?;
            let values: Vec<&str> = xsd_children(restriction, "enumeration")
                .filter_map(|facet| facet.attribute("value"))
                .collect();
            let base = restriction
                .attribute("base")
                .map(|base| qname(restriction, base));
            if !values.is_empty() {
                let type_name = name
                    .or_else(|| base.map(|base| base.local))
                    .unwrap_or("string");
                let joined = values.join(",");
                let (shown, clipped) = clip(&joined, MAX_ENUMERATION_TEXT);
                let more = if clipped { "..." } else { "" };
                return Some(format!("type: {type_name} - enumeration: [{shown}{more}]"));
            }
            node = match base {
                Some(base) => self.schemas.simple_type(base)?,
                None => first_xsd_child(restriction, &["simpleType"])?,
            };
        }
        None
    }

    fn content(&self, type_node: Node<'a, 'input>, hops: usize) -> Content<'a, 'input> {
        if first_xsd_child(type_node, &["simpleContent"]).is_some() {
            return Content::Text;
        }
        let Some(complex_content) = first_xsd_child(type_node, &["complexContent"]) else {
            return match first_xsd_child(type_node, &MODEL_GROUPS) {
                Some(group) => Content::Particles(vec![group]),
                None if type_node.attribute("mixed") == Some("true") => Content::Text,
                None => Content::Empty,
            };
        };
        let Some(derivation) = first_xsd_child(complex_content, &["extension", "restriction"])
        else {
            return Content::Empty;
        };
        let own = first_xsd_child(derivation, &MODEL_GROUPS);
        if is_xsd(derivation, "extension") {
            let mut particles = match self.base_type(derivation) {
                Some(base) if hops < MAX_HOPS => match self.content(base, hops + 1) {
                    Content::Particles(inherited) => inherited,
                    Content::Empty | Content::Text => Vec::new(),
                },
                _ => Vec::new(),
            };
            particles.extend(own);
            return if particles.is_empty() {
                Content::Empty
            } else {
                Content::Particles(particles)
            };
        }
        // A restriction of a built-in type such as SOAP encoding's Array has no particles of
        // its own to show, only a value.
        let restricts_builtin = derivation.attribute("base").is_some_and(|base| {
            let base = qname(derivation, base);
            is_builtin(base.ns) && base.local != "anyType"
        });
        match own {
            Some(group) => Content::Particles(vec![group]),
            None if restricts_builtin => Content::Text,
            None => Content::Empty,
        }
    }

    fn base_type(&self, derivation: Node<'a, 'input>) -> Option<Node<'a, 'input>> {
        let base = derivation.attribute("base")?;
        self.schemas.complex_type(qname(derivation, base))
    }

    /// Attributes of a type, its base type's first; a redeclared name keeps one place.
    fn type_attributes(
        &mut self,
        type_node: Node<'a, 'input>,
        list: &mut Vec<QName<'a>>,
        hops: usize,
    ) {
        let derivation = first_xsd_child(type_node, &["simpleContent", "complexContent"])
            .and_then(|content| first_xsd_child(content, &["extension", "restriction"]));
        let Some(derivation) = derivation else {
            self.own_attributes(type_node, list, hops);
            return;
        };
        if let Some(base) = self.base_type(derivation)
            && hops < MAX_HOPS
        {
            self.type_attributes(base, list, hops + 1);
        }
        self.own_attributes(derivation, list, hops);
    }

    fn own_attributes(
        &mut self,
        container: Node<'a, 'input>,
        list: &mut Vec<QName<'a>>,
        hops: usize,
    ) {
        for child in container.children() {
            self.steps += 1;
            if self.steps > MAX_STEPS {
                return;
            }
            if is_xsd(child, "attribute") {
                let Some(name) = attribute_name(child) else {
                    continue;
                };
                list.retain(|listed| *listed != name);
                if child.attribute("use") != Some("prohibited") {
                    list.push(name);
                }
            } else if is_xsd(child, "attributeGroup") && hops < MAX_HOPS {
                let group = child
                    .attribute("ref")
                    .and_then(|reference| self.schemas.attribute_group(qname(child, reference)));
                if let Some(group) = group {
                    self.own_attributes(group, list, hops + 1);
                }
            }
        }
    }

    fn tag(&mut self, name: QName) -> String {
        match self.prefixes.prefix(name.ns) {
            Some(prefix) => format!("{prefix}:{}", name.local),
            None => name.local.to_owned(),
        }
    }
}

fn type_of<'a, 'input>(declaration: Node<'a, 'input>) -> TypeRef<'a, 'input> {
    if let Some(type_name) = declaration.attribute("type") {
        return TypeRef::Named(qname(declaration, type_name));
    }
    first_xsd_child(declaration, &["complexType", "simpleType"])
        .map_or(TypeRef::Any, TypeRef::Anonymous)
}

/// The attribute's name; `None` for references to standard attributes other than `xml:*`,
/// such as SOAP encoding's arrayType, which samples leave out.
fn attribute_name<'a>(declaration: Node<'a, '_>) -> Option<QName<'a>> {
    if let Some(reference) = declaration.attribute("ref") {
        let name = qname(declaration, reference);
        return (name.ns == XML_NS || !is_builtin(name.ns)).then_some(name);
    }
    let local = declaration.attribute("name")?;
    let ns = if is_qualified(declaration, "attributeFormDefault") {
        target_namespace(declaration)
    } else {
        ""
    };
    Some(QName { ns, local })
}

fn occurrence_comment(min: u64, max: MaxOccurs) -> Option<String> {
    match (min, max) {
        (0, MaxOccurs::Bounded(1)) => Some("Optional:".to_owned()),
        (0, MaxOccurs::Unbounded) => Some("Zero or more repetitions:".to_owned()),
        (min, MaxOccurs::Unbounded) => Some(format!("{min} or more repetitions:")),
        (min, MaxOccurs::Bounded(max)) if max > 1 => Some(format!("{min} to {max} repetitions:")),
        _ => None,
    }
}

/// Prefixes declared on the Envelope, in order of first use; the first is the Envelope's own.
struct Prefixes(Vec<(String, String)>);

impl Prefixes {
    /// `None` for no namespace, which unqualified elements and attributes are in.
    fn prefix(&mut self, ns: &str) -> Option<String> {
        if ns.is_empty() {
            return None;
        }
        if ns == XML_NS {
            return Some("xml".to_owned());
        }
        if let Some((prefix, _)) = self.0.iter().find(|(_, declared)| declared == ns) {
            return Some(prefix.clone());
        }
        let prefix = self.unused(ns);
        self.0.push((prefix.clone(), ns.to_owned()));
        Some(prefix)
    }

    /// Three letters from the namespace, as SoapUI does (`tem` for http://tempuri.org/), four
    /// when those are taken, else `ns1`, `ns2`…
    fn unused(&self, ns: &str) -> String {
        let taken = |candidate: &str| {
            RESERVED_PREFIXES.contains(&candidate)
                || self.0.iter().any(|(prefix, _)| prefix == candidate)
        };
        let letters = prefix_letters(ns);
        for length in [3, 4] {
            let candidate: String = letters.chars().take(length).collect();
            if !candidate.is_empty() && !taken(&candidate) {
                return candidate;
            }
        }
        let mut number = 1;
        loop {
            let candidate = format!("ns{number}");
            if !taken(&candidate) {
                return candidate;
            }
            number += 1;
        }
    }
}

/// Lowercase letters from the most telling part of a namespace: its last meaningful path
/// segment, else its host without `www.`.
fn prefix_letters(ns: &str) -> String {
    let (host, path) = match ns.split_once("://") {
        Some((_, rest)) => rest.split_once('/').unwrap_or((rest, "")),
        None => ("", ns.strip_prefix("urn:").unwrap_or(ns)),
    };
    let path = path.split(['?', '#']).next().unwrap_or_default();
    let segment = path
        .split(['/', ':'])
        .rev()
        .find(|segment| is_meaningful(segment))
        .or_else(|| {
            host.strip_prefix("www.")
                .unwrap_or(host)
                .split(['.', ':'])
                .next()
        });
    let letters: String = segment
        .unwrap_or_default()
        .chars()
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_lowercase())
        .collect();
    // Names starting with "xml" are reserved by the Namespaces in XML spec.
    if letters.starts_with("xml") {
        String::new()
    } else {
        letters
    }
}

fn is_meaningful(segment: &str) -> bool {
    let lower = segment.to_ascii_lowercase();
    let is_version = lower.strip_prefix('v').is_some_and(|rest| {
        !rest.is_empty()
            && rest
                .chars()
                .all(|c| c.is_ascii_digit() || matches!(c, '.' | '_'))
    });
    lower.chars().filter(char::is_ascii_alphabetic).count() >= 2
        && !is_version
        && !GENERIC_SEGMENTS.contains(&lower.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derives_short_prefixes_like_soapui() {
        assert_eq!(prefix_letters("http://tempuri.org/"), "tempuri");
        assert_eq!(prefix_letters("http://www.webserviceX.NET"), "webservicex");
        assert_eq!(
            prefix_letters("http://example.com/shop/orders/v2"),
            "orders"
        );
        assert_eq!(prefix_letters("urn:examples:helloservice"), "helloservice");
        assert_eq!(prefix_letters("http://example.com/calc/xsd"), "calc");
        assert_eq!(prefix_letters("http://127.0.0.1/"), "");

        let mut prefixes = Prefixes(vec![("soapenv".to_owned(), SOAP11_ENVELOPE.to_owned())]);
        assert_eq!(
            prefixes.prefix("http://tempuri.org/").as_deref(),
            Some("tem")
        );
        assert_eq!(
            prefixes.prefix("http://tempuri.org/").as_deref(),
            Some("tem")
        );
        assert_eq!(prefixes.prefix("urn:temperature").as_deref(), Some("temp"));
        assert_eq!(prefixes.prefix("urn:tempest").as_deref(), Some("ns1"));
        assert_eq!(prefixes.prefix("urn:1").as_deref(), Some("ns2"));
        assert_eq!(prefixes.prefix(XML_NS).as_deref(), Some("xml"));
        assert_eq!(prefixes.prefix(""), None);
    }

    #[test]
    fn describes_occurrences_like_soapui() {
        let text = |min, max| occurrence_comment(min, max).unwrap_or_default();
        assert_eq!(text(0, MaxOccurs::Bounded(1)), "Optional:");
        assert_eq!(text(0, MaxOccurs::Unbounded), "Zero or more repetitions:");
        assert_eq!(text(1, MaxOccurs::Unbounded), "1 or more repetitions:");
        assert_eq!(text(2, MaxOccurs::Bounded(5)), "2 to 5 repetitions:");
        assert_eq!(text(1, MaxOccurs::Bounded(1)), "");
    }
}
