import { fileName } from '../body';
import { jsLocal as local, jsText as text } from './js';
import { hasDuplicateNames, sentHeaders, type CodeRequest } from './model';
import { usedVariables, type CodeValues } from './values';

const INDENT = '  ';

export function fetchCode(request: CodeRequest, values: CodeValues): string {
  const body = request.body;
  const readsFiles =
    body?.mode === 'binary' || (body?.mode === 'formData' && body.parts.some((part) => part.file));
  const setup: string[] = [];
  const signing = request.signing;
  if (signing?.kind === 'digest') {
    setup.push('// Digest auth: fetch cannot answer the server’s challenge; use a Digest client.');
  } else if (signing?.kind === 'aws') {
    setup.push('// AWS Signature: sign this request with the AWS SDK (@aws-sdk/signature-v4).');
  }
  if (request.query.length > 0) {
    setup.push(`const url = new URL(${text(values, request.url)});`);
    for (const pair of request.query) {
      setup.push(
        `url.searchParams.append(${text(values, pair.name)}, ${text(values, pair.value)});`,
      );
    }
  } else {
    setup.push(`const url = ${text(values, request.url)};`);
  }

  const headers = sentHeaders(request).map((header): [string, string] => [
    text(values, header.name),
    text(values, header.value),
  ]);
  if (request.basic) {
    const pair = text(values, `${request.basic.username}:${request.basic.password}`);
    headers.unshift([JSON.stringify('Authorization'), `\`Basic \${btoa(${pair})}\``]);
  }
  if (headers.length > 0) {
    // An object would keep only the last of two headers with the same name.
    const duplicates = hasDuplicateNames(sentHeaders(request));
    setup.push(
      duplicates ? 'const headers = [' : 'const headers = {',
      ...headers.map(([name, value]) =>
        duplicates ? `${INDENT}[${name}, ${value}],` : `${INDENT}${name}: ${value},`,
      ),
      duplicates ? '];' : '};',
    );
  }

  let bodyOption: string | null = null;
  if (body?.mode === 'raw') {
    setup.push(`const body = ${text(values, body.text)};`);
    bodyOption = 'body';
  } else if (body?.mode === 'urlencoded') {
    setup.push(
      'const body = new URLSearchParams([',
      ...body.fields.map(
        (field) => `${INDENT}[${text(values, field.name)}, ${text(values, field.value)}],`,
      ),
      ']);',
    );
    bodyOption = 'body';
  } else if (body?.mode === 'formData') {
    setup.push('const form = new FormData();');
    for (const part of body.parts) {
      setup.push(
        part.file
          ? `form.append(${text(values, part.name)}, new Blob([await readFile(${JSON.stringify(part.value)})]), ${JSON.stringify(fileName(part.value))});`
          : `form.append(${text(values, part.name)}, ${text(values, part.value)});`,
      );
    }
    bodyOption = 'body: form';
  } else if (body?.mode === 'binary') {
    setup.push(`const body = await readFile(${JSON.stringify(body.path)});`);
    bodyOption = 'body';
  }
  const sections: string[][] = [];
  const environment = usedVariables(values.variables, setup.join('\n'), local);
  const intro: string[] = [];
  if (environment.length > 0 || readsFiles) {
    intro.push('// Node.js 18 or later, saved as an .mjs file.');
  }
  if (readsFiles) {
    intro.push('import { readFile } from "node:fs/promises";');
  }
  if (intro.length > 0) {
    sections.push(intro);
  }
  if (environment.length > 0) {
    sections.push(
      environment.map((variable) => `const ${variable.camel} = process.env.${variable.env};`),
    );
  }
  sections.push(setup);

  const options = [
    ...(request.method === 'GET' ? [] : [`method: ${JSON.stringify(request.method)}`]),
    ...(headers.length > 0 ? ['headers'] : []),
    ...(bodyOption ? [bodyOption] : []),
  ];
  sections.push([
    options.length === 0
      ? 'const response = await fetch(url);'
      : `const response = await fetch(url, {\n${options.map((option) => `${INDENT}${option},`).join('\n')}\n});`,
    'console.log(response.status);',
    'console.log(await response.text());',
  ]);
  return sections.map((section) => section.join('\n')).join('\n\n');
}
