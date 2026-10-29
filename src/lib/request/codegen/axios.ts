import { fileName } from '../body';
import { jsLocal as local, jsText as text } from './js';
import { mergedHeaders, sentHeaders, type CodeRequest } from './model';
import { usedVariables, type CodeValues } from './values';

const INDENT = '  ';

export function axios(request: CodeRequest, values: CodeValues): string {
  const body = request.body;
  const readsFiles =
    body?.mode === 'binary' || (body?.mode === 'formData' && body.parts.some((part) => part.file));
  const setup: string[] = [];
  const options = [
    `url: ${text(values, request.url)}`,
    `method: ${JSON.stringify(request.method)}`,
  ];

  if (request.query.length > 0) {
    // URLSearchParams keeps the order and repeated names, which a plain object would not.
    setup.push(
      'const params = new URLSearchParams([',
      ...request.query.map(
        (pair) => `${INDENT}[${text(values, pair.name)}, ${text(values, pair.value)}],`,
      ),
      ']);',
    );
    options.push('params');
  }
  const signing = request.signing;
  if (request.basic) {
    options.push(
      `auth: { username: ${text(values, request.basic.username)}, password: ${text(values, request.basic.password)} }`,
    );
  } else if (signing?.kind === 'digest') {
    setup.push('// Digest auth: axios cannot answer the server’s challenge; use a Digest client.');
  } else if (signing?.kind === 'aws') {
    setup.push('// AWS Signature: sign this request with aws4-axios.');
  }
  // An object keeps one value per name, so repeated headers are sent comma-separated.
  const headers = mergedHeaders(sentHeaders(request));
  if (headers.length > 0) {
    setup.push(
      'const headers = {',
      ...headers.map(
        (header) => `${INDENT}${text(values, header.name)}: ${text(values, header.value)},`,
      ),
      '};',
    );
    options.push('headers');
  }

  if (body?.mode === 'raw') {
    setup.push(`const data = ${text(values, body.text)};`);
    // axios would trim a JSON body, or encode it again when it is not valid JSON.
    options.push('data', 'transformRequest: [(body) => body]');
  } else if (body?.mode === 'urlencoded') {
    setup.push(
      'const data = new URLSearchParams([',
      ...body.fields.map(
        (field) => `${INDENT}[${text(values, field.name)}, ${text(values, field.value)}],`,
      ),
      ']);',
    );
    options.push('data');
  } else if (body?.mode === 'formData') {
    setup.push('const data = new FormData();');
    for (const part of body.parts) {
      setup.push(
        part.file
          ? `data.append(${text(values, part.name)}, new Blob([await readFile(${JSON.stringify(part.value)})]), ${JSON.stringify(fileName(part.value))});`
          : `data.append(${text(values, part.name)}, ${text(values, part.value)});`,
      );
    }
    options.push('data');
  } else if (body?.mode === 'binary') {
    setup.push(`const data = await readFile(${JSON.stringify(body.path)});`);
    options.push('data');
  }
  // The body comes back as text, and error statuses are returned instead of thrown.
  options.push('responseType: "text"', 'validateStatus: () => true');

  const sections: string[][] = [
    [
      '// Node.js 18 or later with axios (npm install axios), saved as an .mjs file.',
      'import axios from "axios";',
      ...(readsFiles ? ['import { readFile } from "node:fs/promises";'] : []),
    ],
  ];
  const environment = usedVariables(values.variables, [...setup, ...options].join('\n'), local);
  if (environment.length > 0) {
    sections.push(
      environment.map((variable) => `const ${variable.camel} = process.env.${variable.env};`),
    );
  }
  if (setup.length > 0) {
    sections.push(setup);
  }
  sections.push([
    'const response = await axios.request({',
    ...options.map((option) => `${INDENT}${option},`),
    '});',
    'console.log(response.status);',
    'console.log(response.data);',
  ]);
  return sections.map((section) => section.join('\n')).join('\n\n');
}
