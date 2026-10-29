export type ScriptStage = 'preRequestScript' | 'testScript';

export interface Snippet {
  label: string;
  code: string;
}

const PRE_REQUEST_SNIPPETS: Snippet[] = [
  {
    label: 'Set a variable',
    code: "pm.variables.set('timestamp', Date.now());",
  },
  {
    label: 'Save to environment',
    code: "pm.environment.set('requestId', String(Math.random()).slice(2));",
  },
  {
    label: 'Add a header',
    code: "pm.request.headers.upsert({ key: 'X-Request-Id', value: String(Date.now()) });",
  },
  {
    label: 'Log a variable',
    code: "console.log('baseUrl is', pm.variables.get('baseUrl'));",
  },
  {
    label: 'Fetch a token first',
    code: "pm.sendRequest({\n  url: pm.variables.replaceIn('{{baseUrl}}/auth/token'),\n  method: 'POST',\n  header: [{ key: 'Content-Type', value: 'application/json' }],\n  body: { mode: 'raw', raw: JSON.stringify({ clientId: pm.variables.get('clientId') }) },\n}, (error, res) => {\n  if (error) throw error;\n  pm.variables.set('authToken', res.json().token);\n});",
  },
];

const TEST_SNIPPETS: Snippet[] = [
  {
    label: 'Status code is 200',
    code: "pm.test('status is 200', () => {\n  pm.response.to.have.status(200);\n});",
  },
  {
    label: 'Response time under 500 ms',
    code: "pm.test('response time under 500 ms', () => {\n  pm.expect(pm.response.responseTime).to.be.below(500);\n});",
  },
  {
    label: 'JSON field equals value',
    code: "pm.test('field has the expected value', () => {\n  const body = pm.response.json();\n  pm.expect(body.status).to.equal('ok');\n});",
  },
  {
    label: 'Save field to environment',
    code: "const body = pm.response.json();\npm.environment.set('authToken', body.token);",
  },
  {
    label: 'Header exists',
    code: "pm.test('has Content-Type', () => {\n  pm.response.to.have.header('Content-Type');\n});",
  },
  {
    label: 'SOAP: no fault, read the body',
    code: "pm.test('no SOAP Fault', () => {\n  pm.expect(pm.response.text()).to.not.include('Fault>');\n});\n// Element names stay as written (soap:Envelope); attributes are under $.\nconst envelope = xml2Json(pm.response.text());\nconsole.log(envelope);",
  },
];

export function snippetsFor(stage: ScriptStage): Snippet[] {
  return stage === 'preRequestScript' ? PRE_REQUEST_SNIPPETS : TEST_SNIPPETS;
}

/** Appends a snippet on its own line, keeping existing code untouched. */
export function appendSnippet(script: string, snippet: Snippet): string {
  const trimmed = script.trimEnd();
  return trimmed === '' ? `${snippet.code}\n` : `${trimmed}\n\n${snippet.code}\n`;
}
