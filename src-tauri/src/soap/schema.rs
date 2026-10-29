//! The XML Schema components of a WSDL (its `types` and the schemas they import), looked up by
//! qualified name.

use std::collections::HashMap;

use roxmltree::{Document, Node};

use super::xml::{QName, WSDL_NS, children, is_in, is_xsd, xsd_children, xsd_name};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaxOccurs {
    Bounded(u64),
    Unbounded,
}

/// Global components; when two schemas define the same name, the first one read wins.
#[derive(Default)]
pub struct Schemas<'a, 'input> {
    elements: HashMap<QName<'a>, Node<'a, 'input>>,
    complex_types: HashMap<QName<'a>, Node<'a, 'input>>,
    simple_types: HashMap<QName<'a>, Node<'a, 'input>>,
    groups: HashMap<QName<'a>, Node<'a, 'input>>,
    attribute_groups: HashMap<QName<'a>, Node<'a, 'input>>,
}

impl<'a, 'input> Schemas<'a, 'input> {
    pub fn new(documents: &'a [Document<'input>]) -> Self {
        let mut schemas = Self::default();
        for schema in documents
            .iter()
            .flat_map(|document| schema_nodes(document.root_element()))
        {
            let ns = schema.attribute("targetNamespace").unwrap_or_default();
            for component in schema.children() {
                let Some(local) = component.attribute("name") else {
                    continue;
                };
                let index = match xsd_name(component) {
                    Some("element") => &mut schemas.elements,
                    Some("complexType") => &mut schemas.complex_types,
                    Some("simpleType") => &mut schemas.simple_types,
                    Some("group") => &mut schemas.groups,
                    Some("attributeGroup") => &mut schemas.attribute_groups,
                    _ => continue,
                };
                index.entry(QName { ns, local }).or_insert(component);
            }
        }
        schemas
    }

    pub fn element(&self, name: QName<'a>) -> Option<Node<'a, 'input>> {
        self.elements.get(&name).copied()
    }

    pub fn complex_type(&self, name: QName<'a>) -> Option<Node<'a, 'input>> {
        self.complex_types.get(&name).copied()
    }

    pub fn simple_type(&self, name: QName<'a>) -> Option<Node<'a, 'input>> {
        self.simple_types.get(&name).copied()
    }

    pub fn group(&self, name: QName<'a>) -> Option<Node<'a, 'input>> {
        self.groups.get(&name).copied()
    }

    pub fn attribute_group(&self, name: QName<'a>) -> Option<Node<'a, 'input>> {
        self.attribute_groups.get(&name).copied()
    }
}

/// The schemas in a document: a schema file itself, or those inside a WSDL's `types`.
pub fn schema_nodes<'a, 'input>(root: Node<'a, 'input>) -> Vec<Node<'a, 'input>> {
    if is_xsd(root, "schema") {
        return vec![root];
    }
    if !is_in(root, WSDL_NS, "definitions") {
        return Vec::new();
    }
    children(root, WSDL_NS, "types")
        .flat_map(|types| xsd_children(types, "schema"))
        .collect()
}

pub fn target_namespace<'a>(node: Node<'a, '_>) -> &'a str {
    schema_of(node)
        .and_then(|schema| schema.attribute("targetNamespace"))
        .unwrap_or_default()
}

/// Whether a local element or attribute declaration is in the target namespace (`form`, else
/// the schema's `elementFormDefault` / `attributeFormDefault`).
pub fn is_qualified(declaration: Node, default_attribute: &str) -> bool {
    let form = declaration
        .attribute("form")
        .or_else(|| schema_of(declaration).and_then(|schema| schema.attribute(default_attribute)));
    form.is_some_and(|form| form.trim() == "qualified")
}

pub fn occurs(particle: Node) -> (u64, MaxOccurs) {
    let min = particle
        .attribute("minOccurs")
        .and_then(|value| value.trim().parse().ok())
        .unwrap_or(1);
    let max = match particle.attribute("maxOccurs").map(str::trim) {
        Some("unbounded") => MaxOccurs::Unbounded,
        Some(value) => MaxOccurs::Bounded(value.parse().unwrap_or(1)),
        None => MaxOccurs::Bounded(1),
    };
    (min, max)
}

fn schema_of<'a, 'input>(node: Node<'a, 'input>) -> Option<Node<'a, 'input>> {
    node.ancestors()
        .find(|ancestor| is_xsd(*ancestor, "schema"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::soap::xml::parse;

    #[test]
    fn indexes_global_components_and_reads_occurrences() {
        let text = r#"<wsdl:definitions xmlns:wsdl="http://schemas.xmlsoap.org/wsdl/"
            xmlns:xs="http://www.w3.org/2001/XMLSchema"><wsdl:types>
            <xs:schema targetNamespace="urn:a" elementFormDefault="qualified">
              <xs:element name="A"><xs:complexType><xs:sequence>
                <xs:element name="b" minOccurs="0" maxOccurs="unbounded" form="unqualified"/>
                <xs:element name="c" maxOccurs="3"/>
              </xs:sequence></xs:complexType></xs:element>
              <xs:simpleType name="S"/>
            </xs:schema></wsdl:types></wsdl:definitions>"#;
        let documents = [parse(text).expect("xml")];
        let schemas = Schemas::new(&documents);
        let element = schemas
            .element(QName {
                ns: "urn:a",
                local: "A",
            })
            .expect("global element");
        assert!(
            schemas
                .simple_type(QName {
                    ns: "urn:a",
                    local: "S"
                })
                .is_some()
        );
        let locals: Vec<Node> = element
            .descendants()
            .filter(|node| is_xsd(*node, "element") && *node != element)
            .collect();
        assert_eq!(occurs(locals[0]), (0, MaxOccurs::Unbounded));
        assert_eq!(occurs(locals[1]), (1, MaxOccurs::Bounded(3)));
        assert!(!is_qualified(locals[0], "elementFormDefault"));
        assert!(is_qualified(locals[1], "elementFormDefault"));
        assert_eq!(target_namespace(locals[1]), "urn:a");
    }
}
