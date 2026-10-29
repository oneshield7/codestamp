//! WSDL 1.1: each SOAP binding becomes an interface whose operations carry a sample request.

use std::collections::{BTreeMap, HashMap};

use roxmltree::{Document, Node};

use super::model::{SoapInterface, SoapOperation, SoapVersion, Warnings, WsdlDefinition};
use super::sample::{self, MAX_ENVELOPE_BYTES, Payload, Request, Wrapper, default_encoding_style};
use super::schema::Schemas;
use super::xml::{
    self, QName, SOAP11_BINDING_NS, SOAP12_BINDING_NS, WSDL_NS, WSDL2_NS, children, is_in, qname,
    root_name,
};
use crate::error::AppError;

const DEFAULT_NAME: &str = "SOAP service";
const MAX_OPERATIONS: usize = 2000;
/// Sample text kept for all operations together, so a huge WSDL stays light for the UI.
const MAX_TOTAL_ENVELOPE_BYTES: usize = 32 * 1024 * 1024;

/// A cheap check for the Import page: XML whose root is a WSDL 1.1 `definitions`.
pub fn is_wsdl(text: &str) -> bool {
    root_name(text).is_some_and(|(ns, local)| ns == WSDL_NS && local == "definitions")
}

/// Reads a WSDL from its main document and the documents its imports led to, keyed by their
/// location. Definitions are matched by qualified name across all of them.
pub fn parse_definition(
    main: &str,
    imports: &BTreeMap<String, String>,
) -> Result<WsdlDefinition, AppError> {
    let main = xml::parse(main)?;
    check_root(main.root_element())?;
    let mut warnings = Warnings::default();
    let mut documents = vec![main];
    for (location, text) in imports {
        match xml::parse(text) {
            Ok(document) => documents.push(document),
            Err(error) => warnings.add(format!("Could not read {location}: {error}")),
        }
    }
    let wsdl = Wsdl::new(&documents);
    let interfaces = wsdl.interfaces(&mut warnings);
    Ok(WsdlDefinition {
        name: wsdl.name(),
        target_namespace: wsdl.target_namespace().to_owned(),
        interfaces,
        warnings: warnings.into_vec(),
    })
}

fn check_root(root: Node) -> Result<(), AppError> {
    let name = root.tag_name();
    match (name.namespace(), name.name()) {
        (Some(WSDL_NS), "definitions") => Ok(()),
        (Some(WSDL2_NS), _) => Err(AppError::invalid(
            "WSDL 2.0 is not supported yet; use a WSDL 1.1 document",
        )),
        _ => Err(AppError::invalid("This is not a WSDL document")),
    }
}

/// What is left of the operation and sample limits while reading.
struct Budget {
    operations: usize,
    bytes: usize,
}

/// The parts of a binding its operations need.
struct Binding<'a, 'input> {
    version: SoapVersion,
    soap_ns: &'static str,
    style: &'a str,
    port_type: Option<Node<'a, 'input>>,
}

struct Wsdl<'a, 'input> {
    /// The `definitions` of every document, the main one first.
    definitions: Vec<Node<'a, 'input>>,
    messages: HashMap<QName<'a>, Node<'a, 'input>>,
    port_types: HashMap<QName<'a>, Node<'a, 'input>>,
    schemas: Schemas<'a, 'input>,
}

impl<'a, 'input> Wsdl<'a, 'input> {
    fn new(documents: &'a [Document<'input>]) -> Self {
        let definitions: Vec<Node> = documents
            .iter()
            .map(Document::root_element)
            .filter(|root| is_in(*root, WSDL_NS, "definitions"))
            .collect();
        let mut messages = HashMap::new();
        let mut port_types = HashMap::new();
        for root in &definitions {
            let ns = root.attribute("targetNamespace").unwrap_or_default();
            for child in root.children() {
                let index = if is_in(child, WSDL_NS, "message") {
                    &mut messages
                } else if is_in(child, WSDL_NS, "portType") {
                    &mut port_types
                } else {
                    continue;
                };
                if let Some(local) = child.attribute("name") {
                    index.entry(QName { ns, local }).or_insert(child);
                }
            }
        }
        Self {
            definitions,
            messages,
            port_types,
            schemas: Schemas::new(documents),
        }
    }

    fn name(&self) -> String {
        let service = self
            .definitions
            .iter()
            .flat_map(|root| children(*root, WSDL_NS, "service"))
            .find_map(|service| service.attribute("name"));
        service
            .or_else(|| {
                self.definitions
                    .first()
                    .and_then(|root| root.attribute("name"))
            })
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or(DEFAULT_NAME)
            .to_owned()
    }

    fn target_namespace(&self) -> &'a str {
        self.definitions
            .first()
            .and_then(|root| root.attribute("targetNamespace"))
            .unwrap_or_default()
    }

    fn interfaces(&self, warnings: &mut Warnings) -> Vec<SoapInterface> {
        let mut budget = Budget {
            operations: MAX_OPERATIONS,
            bytes: MAX_TOTAL_ENVELOPE_BYTES,
        };
        self.definitions
            .iter()
            .flat_map(|root| children(*root, WSDL_NS, "binding"))
            .filter_map(|binding| self.interface(binding, &mut budget, warnings))
            .collect()
    }

    /// `None` for bindings that are not SOAP (HTTP GET/POST bindings).
    fn interface(
        &self,
        binding: Node<'a, 'input>,
        budget: &mut Budget,
        warnings: &mut Warnings,
    ) -> Option<SoapInterface> {
        let (version, soap_ns, soap_binding) = soap_binding(binding)?;
        let name = binding.attribute("name")?;
        let port_type = binding
            .attribute("type")
            .and_then(|type_name| self.port_types.get(&qname(binding, type_name)).copied());
        if port_type.is_none() {
            warnings.add(format!(
                "The port type of {name} is not defined, so its samples are empty"
            ));
        }
        let context = Binding {
            version,
            soap_ns,
            style: soap_binding.attribute("style").unwrap_or("document"),
            port_type,
        };
        let mut operations = Vec::new();
        for operation in children(binding, WSDL_NS, "operation") {
            let Some(left) = budget.operations.checked_sub(1) else {
                warnings.add(format!(
                    "Only the first {MAX_OPERATIONS} operations were read"
                ));
                break;
            };
            budget.operations = left;
            operations.extend(self.operation(operation, &context, budget, warnings));
        }
        let qualified = QName {
            ns: target_of(binding),
            local: name,
        };
        Some(SoapInterface {
            name: name.to_owned(),
            version,
            endpoints: self.endpoints(qualified),
            operations,
        })
    }

    fn operation(
        &self,
        operation: Node<'a, 'input>,
        binding: &Binding<'a, 'input>,
        budget: &mut Budget,
        warnings: &mut Warnings,
    ) -> Option<SoapOperation> {
        let name = operation.attribute("name")?;
        let soap_operation = children(operation, binding.soap_ns, "operation").next();
        let action = soap_operation
            .and_then(|node| node.attribute("soapAction"))
            .unwrap_or_default();
        let style = soap_operation
            .and_then(|node| node.attribute("style"))
            .unwrap_or(binding.style);
        let input = children(operation, WSDL_NS, "input").next();
        let abstract_operation = binding.port_type.and_then(|port_type| {
            port_operation(
                port_type,
                name,
                input.and_then(|node| node.attribute("name")),
            )
        });
        let message = self.input_message(abstract_operation, warnings);
        let body = input.and_then(|node| children(node, binding.soap_ns, "body").next());
        let headers: Vec<Node> = input
            .map(|node| children(node, binding.soap_ns, "header").collect())
            .unwrap_or_default();
        let wrapper = (style == "rpc").then(|| Wrapper {
            name,
            namespace: body
                .and_then(|node| node.attribute("namespace"))
                .unwrap_or_else(|| target_of(operation)),
            encoding_style: body
                .filter(|node| node.attribute("use") == Some("encoded"))
                .map(|node| {
                    node.attribute("encodingStyle")
                        .unwrap_or(default_encoding_style(binding.version))
                }),
        });
        let request = Request {
            version: binding.version,
            headers: headers
                .iter()
                .filter_map(|header| self.header_payload(*header, warnings))
                .collect(),
            body: body_payloads(message, body, &headers),
            wrapper,
        };
        let limit = MAX_ENVELOPE_BYTES.min(budget.bytes);
        let envelope = sample::envelope(&self.schemas, &request, name, limit, warnings);
        budget.bytes = budget.bytes.saturating_sub(envelope.len());
        Some(SoapOperation {
            name: name.to_owned(),
            action: action.to_owned(),
            documentation: abstract_operation.map(documentation).unwrap_or_default(),
            envelope,
        })
    }

    fn input_message(
        &self,
        abstract_operation: Option<Node<'a, 'input>>,
        warnings: &mut Warnings,
    ) -> Option<Node<'a, 'input>> {
        let input = children(abstract_operation?, WSDL_NS, "input").next()?;
        let name = qname(input, input.attribute("message")?);
        let message = self.messages.get(&name).copied();
        if message.is_none() {
            warnings.add(format!(
                "The message {} is not defined, so its sample is empty",
                name.local
            ));
        }
        message
    }

    fn header_payload(
        &self,
        header: Node<'a, 'input>,
        warnings: &mut Warnings,
    ) -> Option<Payload<'a>> {
        let name = qname(header, header.attribute("message")?);
        let part = header.attribute("part")?;
        let Some(message) = self.messages.get(&name) else {
            warnings.add(format!("The header message {} is not defined", name.local));
            return None;
        };
        children(*message, WSDL_NS, "part")
            .find(|candidate| candidate.attribute("name") == Some(part))
            .map(payload)
    }

    fn endpoints(&self, binding: QName<'a>) -> Vec<String> {
        let mut endpoints: Vec<String> = Vec::new();
        let ports = self
            .definitions
            .iter()
            .flat_map(|root| children(*root, WSDL_NS, "service"))
            .flat_map(|service| children(service, WSDL_NS, "port"));
        for port in ports {
            if port
                .attribute("binding")
                .is_none_or(|name| qname(port, name) != binding)
            {
                continue;
            }
            let location = port
                .children()
                .find(|child| {
                    is_in(*child, SOAP11_BINDING_NS, "address")
                        || is_in(*child, SOAP12_BINDING_NS, "address")
                })
                .and_then(|address| address.attribute("location"))
                .map(str::trim);
            if let Some(location) = location
                && !location.is_empty()
                && !endpoints.iter().any(|known| known == location)
            {
                endpoints.push(location.to_owned());
            }
        }
        endpoints
    }
}

fn soap_binding<'a, 'input>(
    binding: Node<'a, 'input>,
) -> Option<(SoapVersion, &'static str, Node<'a, 'input>)> {
    binding.children().find_map(|child| {
        if is_in(child, SOAP11_BINDING_NS, "binding") {
            Some((SoapVersion::V11, SOAP11_BINDING_NS, child))
        } else if is_in(child, SOAP12_BINDING_NS, "binding") {
            Some((SoapVersion::V12, SOAP12_BINDING_NS, child))
        } else {
            None
        }
    })
}

fn target_of<'a>(node: Node<'a, '_>) -> &'a str {
    node.document()
        .root_element()
        .attribute("targetNamespace")
        .unwrap_or_default()
}

fn port_operation<'a, 'input>(
    port_type: Node<'a, 'input>,
    name: &str,
    input_name: Option<&str>,
) -> Option<Node<'a, 'input>> {
    let named: Vec<Node> = children(port_type, WSDL_NS, "operation")
        .filter(|operation| operation.attribute("name") == Some(name))
        .collect();
    // Overloaded operations share a name; the binding names the input to tell them apart.
    let by_input = input_name.and_then(|input_name| {
        named.iter().copied().find(|operation| {
            children(*operation, WSDL_NS, "input")
                .next()
                .and_then(|input| input.attribute("name"))
                == Some(input_name)
        })
    });
    by_input.or_else(|| named.first().copied())
}

fn documentation(operation: Node) -> String {
    children(operation, WSDL_NS, "documentation")
        .next()
        .map(|node| {
            let text: String = node
                .descendants()
                .filter(Node::is_text)
                .filter_map(|text| text.text())
                .collect();
            text.trim().to_owned()
        })
        .unwrap_or_default()
}

/// The message parts that go into the Body: those `soap:body@parts` lists, else every part not
/// already bound to a header.
fn body_payloads<'a>(
    message: Option<Node<'a, '_>>,
    body: Option<Node<'a, '_>>,
    headers: &[Node<'a, '_>],
) -> Vec<Payload<'a>> {
    let Some(message) = message else {
        return Vec::new();
    };
    let message_name = QName {
        ns: target_of(message),
        local: message.attribute("name").unwrap_or_default(),
    };
    let listed: Option<Vec<&str>> = body
        .and_then(|node| node.attribute("parts"))
        .map(|parts| parts.split_whitespace().collect());
    let in_headers: Vec<&str> = headers
        .iter()
        .filter(|header| {
            header
                .attribute("message")
                .is_some_and(|name| qname(**header, name) == message_name)
        })
        .filter_map(|header| header.attribute("part"))
        .collect();
    children(message, WSDL_NS, "part")
        .filter(|part| {
            let name = part.attribute("name").unwrap_or_default();
            match &listed {
                Some(listed) => listed.contains(&name),
                None => !in_headers.contains(&name),
            }
        })
        .map(payload)
        .collect()
}

fn payload<'a>(part: Node<'a, '_>) -> Payload<'a> {
    if let Some(element) = part.attribute("element") {
        return Payload::Element(qname(part, element));
    }
    Payload::Typed {
        name: part.attribute("name").unwrap_or("part"),
        type_name: part.attribute("type").map(|name| qname(part, name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CALCULATOR: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<wsdl:definitions xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/" xmlns:tns="http://tempuri.org/"
    xmlns:s="http://www.w3.org/2001/XMLSchema" xmlns:soap12="http://schemas.xmlsoap.org/wsdl/soap12/"
    xmlns:http="http://schemas.xmlsoap.org/wsdl/http/" targetNamespace="http://tempuri.org/"
    xmlns:wsdl="http://schemas.xmlsoap.org/wsdl/">
  <wsdl:types>
    <s:schema elementFormDefault="qualified" targetNamespace="http://tempuri.org/">
      <s:element name="Add">
        <s:complexType>
          <s:sequence>
            <s:element minOccurs="1" maxOccurs="1" name="intA" type="s:int" />
            <s:element minOccurs="1" maxOccurs="1" name="intB" type="s:int" />
          </s:sequence>
        </s:complexType>
      </s:element>
      <s:element name="AddResponse">
        <s:complexType>
          <s:sequence>
            <s:element minOccurs="1" maxOccurs="1" name="AddResult" type="s:int" />
          </s:sequence>
        </s:complexType>
      </s:element>
    </s:schema>
  </wsdl:types>
  <wsdl:message name="AddSoapIn"><wsdl:part name="parameters" element="tns:Add" /></wsdl:message>
  <wsdl:message name="AddSoapOut"><wsdl:part name="parameters" element="tns:AddResponse" /></wsdl:message>
  <wsdl:message name="AddHttpGetIn"><wsdl:part name="intA" type="s:string" /></wsdl:message>
  <wsdl:portType name="CalculatorSoap">
    <wsdl:operation name="Add">
      <wsdl:documentation xmlns:wsdl="http://schemas.xmlsoap.org/wsdl/">
        Adds two integers.
      </wsdl:documentation>
      <wsdl:input message="tns:AddSoapIn" />
      <wsdl:output message="tns:AddSoapOut" />
    </wsdl:operation>
  </wsdl:portType>
  <wsdl:portType name="CalculatorHttpGet">
    <wsdl:operation name="Add"><wsdl:input message="tns:AddHttpGetIn" /></wsdl:operation>
  </wsdl:portType>
  <wsdl:binding name="CalculatorSoap" type="tns:CalculatorSoap">
    <soap:binding transport="http://schemas.xmlsoap.org/soap/http" />
    <wsdl:operation name="Add">
      <soap:operation soapAction="http://tempuri.org/Add" style="document" />
      <wsdl:input><soap:body use="literal" /></wsdl:input>
      <wsdl:output><soap:body use="literal" /></wsdl:output>
    </wsdl:operation>
  </wsdl:binding>
  <wsdl:binding name="CalculatorSoap12" type="tns:CalculatorSoap">
    <soap12:binding transport="http://schemas.xmlsoap.org/soap/http" />
    <wsdl:operation name="Add">
      <soap12:operation soapAction="http://tempuri.org/Add" style="document" />
      <wsdl:input><soap12:body use="literal" /></wsdl:input>
      <wsdl:output><soap12:body use="literal" /></wsdl:output>
    </wsdl:operation>
  </wsdl:binding>
  <wsdl:binding name="CalculatorHttpGet" type="tns:CalculatorHttpGet">
    <http:binding verb="GET" />
    <wsdl:operation name="Add"><http:operation location="/Add" /></wsdl:operation>
  </wsdl:binding>
  <wsdl:service name="Calculator">
    <wsdl:port name="CalculatorSoap" binding="tns:CalculatorSoap">
      <soap:address location="https://calc.example.com/calculator.asmx" />
    </wsdl:port>
    <wsdl:port name="CalculatorSoap12" binding="tns:CalculatorSoap12">
      <soap12:address location="https://calc.example.com/calculator.asmx" />
    </wsdl:port>
  </wsdl:service>
</wsdl:definitions>"#;

    const ADD_11: &str = "<soapenv:Envelope xmlns:soapenv=\"http://schemas.xmlsoap.org/soap/envelope/\" xmlns:tem=\"http://tempuri.org/\">
  <soapenv:Header/>
  <soapenv:Body>
    <tem:Add>
      <tem:intA>?</tem:intA>
      <tem:intB>?</tem:intB>
    </tem:Add>
  </soapenv:Body>
</soapenv:Envelope>";

    const ADD_12: &str = "<soap:Envelope xmlns:soap=\"http://www.w3.org/2003/05/soap-envelope\" xmlns:tem=\"http://tempuri.org/\">
  <soap:Header/>
  <soap:Body>
    <tem:Add>
      <tem:intA>?</tem:intA>
      <tem:intB>?</tem:intB>
    </tem:Add>
  </soap:Body>
</soap:Envelope>";

    fn definition(main: &str) -> WsdlDefinition {
        parse_definition(main, &BTreeMap::new()).expect("definition")
    }

    #[test]
    fn reads_a_dotnet_calculator_with_both_soap_versions() {
        let definition = definition(CALCULATOR);
        assert_eq!(definition.name, "Calculator");
        assert_eq!(definition.target_namespace, "http://tempuri.org/");
        assert!(definition.warnings.is_empty(), "{:?}", definition.warnings);
        let interfaces: Vec<_> = definition
            .interfaces
            .iter()
            .map(|interface| (interface.name.as_str(), interface.version))
            .collect();
        assert_eq!(
            interfaces,
            [
                ("CalculatorSoap", SoapVersion::V11),
                ("CalculatorSoap12", SoapVersion::V12)
            ]
        );
        let soap11 = &definition.interfaces[0];
        assert_eq!(
            soap11.endpoints,
            ["https://calc.example.com/calculator.asmx"]
        );
        let add = &soap11.operations[0];
        assert_eq!(add.name, "Add");
        assert_eq!(add.action, "http://tempuri.org/Add");
        assert_eq!(add.documentation, "Adds two integers.");
        assert_eq!(add.envelope, ADD_11);
        assert_eq!(definition.interfaces[1].operations[0].envelope, ADD_12);
    }

    #[test]
    fn wraps_rpc_parts_in_the_operation_element() {
        let definition = definition(
            r#"<definitions name="HelloService" targetNamespace="http://www.examples.com/wsdl/HelloService.wsdl"
              xmlns="http://schemas.xmlsoap.org/wsdl/" xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/"
              xmlns:tns="http://www.examples.com/wsdl/HelloService.wsdl" xmlns:xsd="http://www.w3.org/2001/XMLSchema">
              <message name="SayHelloRequest">
                <part name="firstName" type="xsd:string"/>
                <part name="age" type="xsd:int"/>
              </message>
              <portType name="Hello_PortType">
                <operation name="sayHello"><input message="tns:SayHelloRequest"/></operation>
                <operation name="sayHi"><input message="tns:SayHelloRequest"/></operation>
              </portType>
              <binding name="Hello_Binding" type="tns:Hello_PortType">
                <soap:binding style="rpc" transport="http://schemas.xmlsoap.org/soap/http"/>
                <operation name="sayHello">
                  <soap:operation soapAction="sayHello"/>
                  <input><soap:body namespace="urn:examples:helloservice" use="literal"/></input>
                </operation>
                <operation name="sayHi">
                  <input><soap:body namespace="urn:examples:helloservice" use="encoded"
                    encodingStyle="http://schemas.xmlsoap.org/soap/encoding/"/></input>
                </operation>
              </binding>
            </definitions>"#,
        );
        assert_eq!(definition.name, "HelloService");
        let operations = &definition.interfaces[0].operations;
        assert_eq!(operations[0].action, "sayHello");
        assert_eq!(operations[1].action, "");
        assert_eq!(
            operations[0].envelope,
            "<soapenv:Envelope xmlns:soapenv=\"http://schemas.xmlsoap.org/soap/envelope/\" xmlns:hel=\"urn:examples:helloservice\">
  <soapenv:Header/>
  <soapenv:Body>
    <hel:sayHello>
      <firstName>?</firstName>
      <age>?</age>
    </hel:sayHello>
  </soapenv:Body>
</soapenv:Envelope>"
        );
        assert!(operations[1].envelope.contains(
            "<hel:sayHi soapenv:encodingStyle=\"http://schemas.xmlsoap.org/soap/encoding/\">\n      <firstName>?</firstName>"
        ));
    }

    #[test]
    fn covers_extensions_occurrences_attributes_choices_and_enumerations() {
        let definition = definition(
            r#"<wsdl:definitions xmlns:wsdl="http://schemas.xmlsoap.org/wsdl/"
              xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/" xmlns:xs="http://www.w3.org/2001/XMLSchema"
              xmlns:tns="http://example.com/shop/orders" targetNamespace="http://example.com/shop/orders">
              <wsdl:types>
                <xs:schema targetNamespace="http://example.com/shop/orders" elementFormDefault="qualified">
                  <xs:simpleType name="Color">
                    <xs:restriction base="xs:string">
                      <xs:enumeration value="red"/><xs:enumeration value="green"/>
                    </xs:restriction>
                  </xs:simpleType>
                  <xs:complexType name="Item">
                    <xs:sequence>
                      <xs:element name="sku" type="xs:string"/>
                      <xs:element name="color" type="tns:Color" minOccurs="0"/>
                    </xs:sequence>
                    <xs:attribute name="id" type="xs:int" use="required"/>
                  </xs:complexType>
                  <xs:complexType name="GiftItem">
                    <xs:complexContent>
                      <xs:extension base="tns:Item">
                        <xs:sequence><xs:element name="message" type="xs:string"/></xs:sequence>
                        <xs:attributeGroup ref="tns:Wrapping"/>
                      </xs:extension>
                    </xs:complexContent>
                  </xs:complexType>
                  <xs:attributeGroup name="Wrapping">
                    <xs:attribute name="wrapped" type="xs:boolean"/>
                  </xs:attributeGroup>
                  <xs:complexType name="Price">
                    <xs:simpleContent>
                      <xs:extension base="xs:decimal">
                        <xs:attribute name="currency" type="xs:string"/>
                      </xs:extension>
                    </xs:simpleContent>
                  </xs:complexType>
                  <xs:group name="Payment">
                    <xs:choice>
                      <xs:element name="card" type="xs:string"/>
                      <xs:element name="voucher" type="xs:string"/>
                    </xs:choice>
                  </xs:group>
                  <xs:element name="Auth">
                    <xs:complexType><xs:sequence><xs:element name="token" type="xs:string"/></xs:sequence></xs:complexType>
                  </xs:element>
                  <xs:element name="PlaceOrder">
                    <xs:complexType>
                      <xs:sequence>
                        <xs:element name="item" type="tns:GiftItem" maxOccurs="unbounded"/>
                        <xs:element name="note" type="xs:string" minOccurs="0" maxOccurs="unbounded"/>
                        <xs:element name="total" type="tns:Price"/>
                        <xs:group ref="tns:Payment"/>
                        <xs:element name="hidden" type="xs:string" maxOccurs="0"/>
                        <xs:any minOccurs="0" maxOccurs="unbounded" processContents="lax"/>
                      </xs:sequence>
                      <xs:attribute ref="xml:lang"/>
                    </xs:complexType>
                  </xs:element>
                </xs:schema>
              </wsdl:types>
              <wsdl:message name="PlaceOrderIn">
                <wsdl:part name="auth" element="tns:Auth"/>
                <wsdl:part name="body" element="tns:PlaceOrder"/>
              </wsdl:message>
              <wsdl:portType name="OrdersPort">
                <wsdl:operation name="PlaceOrder"><wsdl:input message="tns:PlaceOrderIn"/></wsdl:operation>
              </wsdl:portType>
              <wsdl:binding name="OrdersBinding" type="tns:OrdersPort">
                <soap:binding style="document" transport="http://schemas.xmlsoap.org/soap/http"/>
                <wsdl:operation name="PlaceOrder">
                  <soap:operation soapAction=""/>
                  <wsdl:input>
                    <soap:header message="tns:PlaceOrderIn" part="auth" use="literal"/>
                    <soap:body use="literal"/>
                  </wsdl:input>
                </wsdl:operation>
              </wsdl:binding>
            </wsdl:definitions>"#,
        );
        assert!(definition.warnings.is_empty(), "{:?}", definition.warnings);
        assert_eq!(definition.name, "SOAP service");
        assert_eq!(
            definition.interfaces[0].operations[0].envelope,
            "<soapenv:Envelope xmlns:soapenv=\"http://schemas.xmlsoap.org/soap/envelope/\" xmlns:ord=\"http://example.com/shop/orders\">
  <soapenv:Header>
    <ord:Auth>
      <ord:token>?</ord:token>
    </ord:Auth>
  </soapenv:Header>
  <soapenv:Body>
    <ord:PlaceOrder xml:lang=\"?\">
      <!--1 or more repetitions:-->
      <ord:item id=\"?\" wrapped=\"?\">
        <ord:sku>?</ord:sku>
        <!--Optional:-->
        <!--type: Color - enumeration: [red,green]-->
        <ord:color>?</ord:color>
        <ord:message>?</ord:message>
      </ord:item>
      <!--Zero or more repetitions:-->
      <ord:note>?</ord:note>
      <ord:total currency=\"?\">?</ord:total>
      <!--You have a CHOICE of the next 2 items at this level-->
      <ord:card>?</ord:card>
      <ord:voucher>?</ord:voucher>
      <!--You may enter ANY elements at this point-->
    </ord:PlaceOrder>
  </soapenv:Body>
</soapenv:Envelope>"
        );
    }

    const TREE: &str = r#"<wsdl:definitions xmlns:wsdl="http://schemas.xmlsoap.org/wsdl/"
      xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/" xmlns:xs="http://www.w3.org/2001/XMLSchema"
      xmlns:tns="http://example.com/tree" targetNamespace="http://example.com/tree">
      <wsdl:types>
        <xs:schema targetNamespace="http://example.com/tree">
          <xs:complexType name="Node">
            <xs:sequence>
              <xs:element name="value" type="xs:string"/>
              <xs:element name="child" type="tns:Node" minOccurs="0"/>
            </xs:sequence>
          </xs:complexType>
          <xs:element name="Tree" type="tns:Node"/>
          <xs:element name="Broken" type="tns:Nowhere"/>
        </xs:schema>
      </wsdl:types>
      <wsdl:message name="In"><wsdl:part name="tree" element="tns:Tree"/></wsdl:message>
      <wsdl:message name="BrokenIn">
        <wsdl:part name="broken" element="tns:Broken"/>
        <wsdl:part name="missing" element="tns:Missing"/>
      </wsdl:message>
      <wsdl:portType name="Port">
        <wsdl:operation name="Plant"><wsdl:input message="tns:In"/></wsdl:operation>
        <wsdl:operation name="Break"><wsdl:input message="tns:BrokenIn"/></wsdl:operation>
      </wsdl:portType>
      <wsdl:binding name="Binding" type="tns:Port">
        <soap:binding transport="http://schemas.xmlsoap.org/soap/http"/>
        <wsdl:operation name="Plant"><wsdl:input><soap:body use="literal"/></wsdl:input></wsdl:operation>
        <wsdl:operation name="Break"><wsdl:input><soap:body use="literal"/></wsdl:input></wsdl:operation>
      </wsdl:binding>
    </wsdl:definitions>"#;

    #[test]
    fn stops_recursive_types_and_reports_undefined_ones() {
        let definition = definition(TREE);
        let operations = &definition.interfaces[0].operations;
        assert!(operations[0].envelope.contains(
            "    <tre:Tree>
      <value>?</value>
      <!--Optional:-->
      <child>
        <value>?</value>
        <!--child is left out: nested too deep-->
      </child>
    </tre:Tree>
"
        ));
        assert!(
            operations[1]
                .envelope
                .contains("<tre:Broken>?</tre:Broken>")
        );
        assert!(
            operations[1]
                .envelope
                .contains("<tre:Missing>?</tre:Missing>")
        );
        assert_eq!(
            definition.warnings,
            [
                "The type Nowhere is not defined; its values show ?",
                "The element Missing is not defined; its sample shows ?"
            ]
        );
    }

    #[test]
    fn finds_definitions_in_imported_documents() {
        let main = r#"<wsdl:definitions xmlns:wsdl="http://schemas.xmlsoap.org/wsdl/"
          xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/" xmlns:t="urn:ping"
          xmlns:tns="urn:svc" targetNamespace="urn:svc">
          <wsdl:import namespace="urn:ping" location="types.xsd"/>
          <wsdl:message name="In"><wsdl:part name="p" element="t:Ping"/></wsdl:message>
          <wsdl:portType name="P"><wsdl:operation name="Ping"><wsdl:input message="tns:In"/></wsdl:operation></wsdl:portType>
          <wsdl:binding name="B" type="tns:P">
            <soap:binding/>
            <wsdl:operation name="Ping"><wsdl:input><soap:body use="literal"/></wsdl:input></wsdl:operation>
          </wsdl:binding>
        </wsdl:definitions>"#;
        let types = r#"<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" targetNamespace="urn:ping">
          <xs:element name="Ping"><xs:complexType/></xs:element>
        </xs:schema>"#;
        let imports = BTreeMap::from([
            ("https://example.com/types.xsd".to_owned(), types.to_owned()),
            (
                "https://example.com/bad.xsd".to_owned(),
                "not xml".to_owned(),
            ),
        ]);
        let definition = parse_definition(main, &imports).expect("definition");
        assert!(
            definition.interfaces[0].operations[0]
                .envelope
                .contains("<soapenv:Body>\n    <pin:Ping/>\n  </soapenv:Body>")
        );
        assert_eq!(
            definition.warnings,
            ["Could not read https://example.com/bad.xsd: The document is not valid XML"]
        );
    }

    #[test]
    fn refuses_other_documents() {
        let message = |text: &str| {
            parse_definition(text, &BTreeMap::new())
                .expect_err("refused")
                .to_string()
        };
        assert_eq!(
            message(r#"<description xmlns="http://www.w3.org/ns/wsdl"/>"#),
            "WSDL 2.0 is not supported yet; use a WSDL 1.1 document"
        );
        assert_eq!(message("<root/>"), "This is not a WSDL document");
        assert_eq!(
            message("<!DOCTYPE d [<!ENTITY e \"x\">]><d>&e;</d>"),
            "XML with a DTD is not supported"
        );
        assert!(is_wsdl(CALCULATOR));
        assert!(!is_wsdl(
            r#"<description xmlns="http://www.w3.org/ns/wsdl"/>"#
        ));
        assert!(!is_wsdl("{ \"openapi\": \"3.1.0\" }"));
    }
}
