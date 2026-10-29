# Stage 24: Body XML dan HTML, SOAP, dan import WSDL

Suite [`stage24-e2e.mjs`](../../../e2e/stage24-e2e.mjs) · 20 cek

Menguji body XML, HTML, dan Binary, response XML dan HTML yang dirapikan, request SOAP dari WSDL (import, SOAP 1.1 dan 1.2, fault, `xml2Json` di test script), tombol SOAP di kepala section, import WSDL menjadi collection, WSDL lewat http biasa di host lain, dan tooltip tile Home. Membuat "E2E SOAP" dan "Calculator" lalu menghapus keduanya.

## Kebutuhan

- Server SOAP uji `soap-server.mjs` (Node).

## Alur

### 1. XML, HTML and Binary bodies

- the body offers XML, HTML and Binary
- Beautify indents XML
- broken XML says where
- Beautify indents HTML and keeps scripts as written

### 2. Formatted XML and HTML responses

- an XML response is shown indented
- an HTML response is shown indented

### 3. A SOAP request from a WSDL

- Add SOAP makes a SOAP card with an Envelope tab
- Load WSDL picks the first operation with its sample, endpoint and action
- the sample envelope reads the types from the imported XSD
- a SOAP Fault is pointed out
- SOAP 1.1 sends text/xml with a quoted SOAPAction
- a filled envelope is answered and xml2Json reads it in tests
- switching operation keeps an edited envelope until Reset
- SOAP 1.2 puts the action in the Content-Type

### 4. The section head offers SOAP too

- the section head has an Add SOAP button

### 5. Importing a WSDL as a collection

- the Import page previews a WSDL
- the import makes a section per binding and a SOAP request per operation

### 6. A WSDL over plain http on another host

- a WSDL over plain http on another host is read with its imports

### 7. Home: the chosen tile hides its tooltip

- a tile shows its name on hover, but not once chosen
- test data cleaned up

---

Sebelumnya: [Orchestration](stage-23.md) · [Daftar stage](../README.md#daftar-stage) · Berikutnya: [Title bar dan sidebar yang diciutkan](stage-25.md)
