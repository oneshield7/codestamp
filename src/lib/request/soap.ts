import type { SoapVersion } from '../api/collections';

const SOAP_11 = 'http://schemas.xmlsoap.org/soap/envelope/';
const SOAP_12 = 'http://www.w3.org/2003/05/soap-envelope';
/** Fault texts beyond this are cut, so a stack trace in a fault does not flood the summary. */
const MAX_FAULT_CHARS = 300;

export interface SoapFault {
  code: string;
  reason: string;
}

/** Whether a file to import is a WSDL (1.1, or 2.0 so it gets a clear "not supported"). */
export function looksLikeWsdl(text: string): boolean {
  const head = text.slice(0, 4096);
  return (
    (head.includes('http://schemas.xmlsoap.org/wsdl/') ||
      head.includes('http://www.w3.org/ns/wsdl')) &&
    /[<:](definitions|description)[\s>]/.test(head)
  );
}

/** A web address that names a WSDL, as `…?wsdl`, `…?singleWsdl` or `….wsdl` do. */
export function isWsdlUrl(url: string): boolean {
  return /^https?:\/\//i.test(url) && /([?&](single)?wsdl(?![a-z])|\.wsdl([?#]|$))/i.test(url);
}

/** The Content-Type of a SOAP request; SOAP 1.2 carries the action in it. */
export function soapContentType(version: SoapVersion, action: string): string {
  if (version === '1.1') {
    return 'text/xml; charset=utf-8';
  }
  return action === ''
    ? 'application/soap+xml; charset=utf-8'
    : `application/soap+xml; charset=utf-8; action="${action}"`;
}

function childText(parent: Element, names: string[]): string {
  for (const child of Array.from(parent.children)) {
    if (names.includes(child.localName)) {
      return child.textContent.trim();
    }
  }
  return '';
}

/** The fault in a SOAP 1.1 or 1.2 response envelope, or null when there is none. */
export function soapFault(text: string): SoapFault | null {
  if (!text.includes('Fault')) {
    return null;
  }
  const doc = new DOMParser().parseFromString(text, 'application/xml');
  if (doc.getElementsByTagName('parsererror').length > 0) {
    return null;
  }
  const envelope = doc.documentElement;
  const namespace = envelope.namespaceURI;
  if (envelope.localName !== 'Envelope' || (namespace !== SOAP_11 && namespace !== SOAP_12)) {
    return null;
  }
  const fault = doc.getElementsByTagNameNS(namespace, 'Fault').item(0);
  if (!fault) {
    return null;
  }
  const clip = (value: string): string =>
    value.length > MAX_FAULT_CHARS ? `${value.slice(0, MAX_FAULT_CHARS)}…` : value;
  // SOAP 1.2 nests the code as Code/Value and the message as Reason/Text.
  const code = childText(fault, ['faultcode', 'Code']).split(/\s+/).at(0) ?? '';
  const reason = childText(fault, ['faultstring', 'Reason']);
  return { code: clip(code), reason: clip(reason.replace(/\s+/g, ' ')) };
}
