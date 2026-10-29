// A small SOAP Calculator (Add, Subtract) with a WSDL whose types live in a separate XSD, a
// SOAP 1.1 and a SOAP 1.2 binding, and faults for values that are not numbers. Also serves
// plain XML and HTML pages for the response viewer. Used by stage24-e2e.mjs.
import http from 'node:http';

const SOAP11 = 'http://schemas.xmlsoap.org/soap/envelope/';
const SOAP12 = 'http://www.w3.org/2003/05/soap-envelope';

const XSD = `<?xml version="1.0" encoding="utf-8"?>
<xs:schema xmlns:xs="http://www.w3.org/2001/XMLSchema" elementFormDefault="qualified" targetNamespace="http://tempuri.org/">
  <xs:element name="Add"><xs:complexType><xs:sequence>
    <xs:element name="intA" type="xs:int"/><xs:element name="intB" type="xs:int"/>
  </xs:sequence></xs:complexType></xs:element>
  <xs:element name="AddResponse"><xs:complexType><xs:sequence>
    <xs:element name="AddResult" type="xs:int"/>
  </xs:sequence></xs:complexType></xs:element>
  <xs:element name="Subtract"><xs:complexType><xs:sequence>
    <xs:element name="intA" type="xs:int"/><xs:element name="intB" type="xs:int"/>
  </xs:sequence></xs:complexType></xs:element>
  <xs:element name="SubtractResponse"><xs:complexType><xs:sequence>
    <xs:element name="SubtractResult" type="xs:int"/>
  </xs:sequence></xs:complexType></xs:element>
</xs:schema>`;

function wsdl(base) {
  const operations = (prefix) =>
    ['Add', 'Subtract']
      .map(
        (name) => `
    <wsdl:operation name="${name}">
      <${prefix}:operation soapAction="http://tempuri.org/${name}" style="document"/>
      <wsdl:input><${prefix}:body use="literal"/></wsdl:input>
      <wsdl:output><${prefix}:body use="literal"/></wsdl:output>
    </wsdl:operation>`,
      )
      .join('');
  return `<?xml version="1.0" encoding="utf-8"?>
<wsdl:definitions xmlns:wsdl="http://schemas.xmlsoap.org/wsdl/" xmlns:soap="http://schemas.xmlsoap.org/wsdl/soap/"
    xmlns:soap12="http://schemas.xmlsoap.org/wsdl/soap12/" xmlns:xs="http://www.w3.org/2001/XMLSchema"
    xmlns:tns="http://tempuri.org/" targetNamespace="http://tempuri.org/">
  <wsdl:documentation>Adds and subtracts two numbers.</wsdl:documentation>
  <wsdl:types>
    <xs:schema><xs:import namespace="http://tempuri.org/" schemaLocation="calculator.xsd"/></xs:schema>
  </wsdl:types>
  <wsdl:message name="AddIn"><wsdl:part name="parameters" element="tns:Add"/></wsdl:message>
  <wsdl:message name="AddOut"><wsdl:part name="parameters" element="tns:AddResponse"/></wsdl:message>
  <wsdl:message name="SubtractIn"><wsdl:part name="parameters" element="tns:Subtract"/></wsdl:message>
  <wsdl:message name="SubtractOut"><wsdl:part name="parameters" element="tns:SubtractResponse"/></wsdl:message>
  <wsdl:portType name="CalculatorSoap">
    <wsdl:operation name="Add"><wsdl:documentation>Adds intA and intB.</wsdl:documentation>
      <wsdl:input message="tns:AddIn"/><wsdl:output message="tns:AddOut"/></wsdl:operation>
    <wsdl:operation name="Subtract"><wsdl:input message="tns:SubtractIn"/><wsdl:output message="tns:SubtractOut"/></wsdl:operation>
  </wsdl:portType>
  <wsdl:binding name="CalculatorSoap" type="tns:CalculatorSoap">
    <soap:binding transport="http://schemas.xmlsoap.org/soap/http"/>${operations('soap')}
  </wsdl:binding>
  <wsdl:binding name="CalculatorSoap12" type="tns:CalculatorSoap">
    <soap12:binding transport="http://schemas.xmlsoap.org/soap/http"/>${operations('soap12')}
  </wsdl:binding>
  <wsdl:service name="Calculator">
    <wsdl:port name="CalculatorSoap" binding="tns:CalculatorSoap"><soap:address location="${base}/calculator.asmx"/></wsdl:port>
    <wsdl:port name="CalculatorSoap12" binding="tns:CalculatorSoap12"><soap12:address location="${base}/calculator.asmx"/></wsdl:port>
  </wsdl:service>
</wsdl:definitions>`;
}

function value(body, name) {
  return new RegExp(`<(?:\\w+:)?${name}>([^<]*)</`).exec(body)?.[1]?.trim() ?? '';
}

function envelope(version, inner) {
  const ns = version === '1.2' ? SOAP12 : SOAP11;
  return `<?xml version="1.0" encoding="utf-8"?><soap:Envelope xmlns:soap="${ns}"><soap:Body>${inner}</soap:Body></soap:Envelope>`;
}

function fault(version, reason) {
  return version === '1.2'
    ? `<soap:Fault><soap:Code><soap:Value>soap:Sender</soap:Value></soap:Code><soap:Reason><soap:Text xml:lang="en">${reason}</soap:Text></soap:Reason></soap:Fault>`
    : `<soap:Fault><faultcode>soap:Client</faultcode><faultstring>${reason}</faultstring></soap:Fault>`;
}

export async function startSoapServer(host = '127.0.0.1') {
  const received = [];
  const fetched = [];
  const server = http.createServer((req, res) => {
    const chunks = [];
    req.on('data', (chunk) => chunks.push(chunk));
    req.on('end', () => {
      const url = new URL(req.url, 'http://localhost');
      const base = `http://${req.headers.host}`;
      if (req.method === 'GET') {
        fetched.push(url.pathname);
      }
      if (
        req.method === 'GET' &&
        url.pathname === '/calculator.asmx' &&
        url.searchParams.has('wsdl')
      ) {
        res.writeHead(200, { 'Content-Type': 'text/xml; charset=utf-8' });
        return res.end(wsdl(base));
      }
      if (req.method === 'GET' && url.pathname === '/calculator.xsd') {
        res.writeHead(200, { 'Content-Type': 'text/xml; charset=utf-8' });
        return res.end(XSD);
      }
      if (req.method === 'GET' && url.pathname === '/plain.xml') {
        res.writeHead(200, { 'Content-Type': 'application/xml' });
        return res.end('<?xml version="1.0"?><root><a x="1">t</a><b/></root>');
      }
      if (req.method === 'GET' && url.pathname === '/page.html') {
        res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
        return res.end(
          '<!DOCTYPE html><html><head><title>T</title></head><body><p>Hi</p></body></html>',
        );
      }
      if (req.method !== 'POST' || url.pathname !== '/calculator.asmx') {
        res.writeHead(404);
        return res.end();
      }
      const body = Buffer.concat(chunks).toString('utf8');
      const contentType = req.headers['content-type'] ?? '';
      const version = contentType.startsWith('application/soap+xml') ? '1.2' : '1.1';
      const action =
        version === '1.2'
          ? (/action="([^"]*)"/.exec(contentType)?.[1] ?? '')
          : (req.headers.soapaction ?? '').replace(/"/g, '');
      received.push({ contentType, soapAction: req.headers.soapaction ?? null, action, body });
      const operation = action.split('/').at(-1);
      const a = Number.parseInt(value(body, 'intA'), 10);
      const b = Number.parseInt(value(body, 'intB'), 10);
      const reply = {
        'Content-Type':
          version === '1.2' ? 'application/soap+xml; charset=utf-8' : 'text/xml; charset=utf-8',
      };
      if (!['Add', 'Subtract'].includes(operation) || Number.isNaN(a) || Number.isNaN(b)) {
        res.writeHead(500, reply);
        return res.end(
          envelope(
            version,
            fault(
              version,
              'Server was unable to read request. Input string was not in a correct format.',
            ),
          ),
        );
      }
      const result = operation === 'Add' ? a + b : a - b;
      res.writeHead(200, reply);
      res.end(
        envelope(
          version,
          `<${operation}Response xmlns="http://tempuri.org/"><${operation}Result>${result}</${operation}Result></${operation}Response>`,
        ),
      );
    });
  });
  await new Promise((resolve) => server.listen(0, host, resolve));
  return { server, received, fetched, base: `http://${host}:${server.address().port}` };
}
