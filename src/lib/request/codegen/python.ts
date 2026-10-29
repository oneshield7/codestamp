import { fileName } from '../body';
import { mergedHeaders, sentHeaders, type CodeRequest } from './model';
import { joined, textSegments, type Segment } from './segments';
import { usedVariables, type CodeValues, type CodeVariable } from './values';

const INDENT = '    ';

const local = (variable: CodeVariable): string => variable.snake;

/** A JSON string literal is also a Python one; an f-string doubles its braces. */
function string(segments: Segment[]): string {
  const parts = joined(segments);
  const [only] = parts;
  if (parts.length === 1 && only?.code !== undefined) {
    return only.code;
  }
  const plain = parts.every((part) => part.text !== undefined);
  const whole = parts.map((part) => part.text ?? '').join('');
  if (whole.includes('\n')) {
    return block(parts, plain);
  }
  if (plain) {
    return JSON.stringify(whole);
  }
  const inner = parts
    .map((part) =>
      part.text !== undefined
        ? JSON.stringify(part.text).slice(1, -1).replace(/[{}]/g, '$&$&')
        : `{${part.code}}`,
    )
    .join('');
  return `f"${inner}"`;
}

/** Triple quotes keep the body's lines as they are. */
function block(parts: Segment[], plain: boolean): string {
  const inner = parts
    .map((part) => {
      if (part.text === undefined) {
        return `{${part.code}}`;
      }
      const escaped = part.text.replace(/\\/g, '\\\\').replace(/"""/g, '""\\"');
      return plain ? escaped : escaped.replace(/[{}]/g, '$&$&');
    })
    .join('')
    .replace(/"$/, '\\"');
  return `${plain ? '' : 'f'}"""${inner}"""`;
}

function text(values: CodeValues, value: string): string {
  return string(textSegments(values, value, local));
}

export function python(request: CodeRequest, values: CodeValues): string {
  const signing = request.signing;
  const setup: string[] = [];
  const args = [JSON.stringify(request.method), 'url'];
  if (signing?.kind === 'aws') {
    setup.push('# AWS Signature: sign this request, for example with requests-aws4auth.');
  }
  setup.push(`url = ${text(values, request.url)}`);
  if (request.query.length > 0) {
    setup.push(
      'params = [',
      ...request.query.map(
        (pair) => `${INDENT}(${text(values, pair.name)}, ${text(values, pair.value)}),`,
      ),
      ']',
    );
    args.push('params=params');
  }
  if (request.basic) {
    args.push(
      `auth=(${text(values, request.basic.username)}, ${text(values, request.basic.password)})`,
    );
  } else if (signing?.kind === 'digest') {
    args.push(
      `auth=HTTPDigestAuth(${text(values, signing.username)}, ${text(values, signing.password)})`,
    );
  }
  // A dict keeps one value per name, so repeated headers are sent comma-separated.
  const headers = mergedHeaders(sentHeaders(request));
  if (headers.length > 0) {
    setup.push(
      'headers = {',
      ...headers.map(
        (header) => `${INDENT}${text(values, header.name)}: ${text(values, header.value)},`,
      ),
      '}',
    );
    args.push('headers=headers');
  }
  const body = request.body;
  let opensFile: string | null = null;
  if (body?.mode === 'raw') {
    setup.push(`payload = ${text(values, body.text)}`);
    args.push('data=payload.encode("utf-8")');
  } else if (body?.mode === 'urlencoded') {
    setup.push(
      'payload = [',
      ...body.fields.map(
        (field) => `${INDENT}(${text(values, field.name)}, ${text(values, field.value)}),`,
      ),
      ']',
    );
    args.push('data=payload');
  } else if (body?.mode === 'formData') {
    // Passing every field in `files` makes requests send multipart even without a file.
    setup.push(
      'files = [',
      ...body.parts.map((part) => {
        const value = part.file
          ? `(${JSON.stringify(fileName(part.value))}, open(${JSON.stringify(part.value)}, "rb"))`
          : `(None, ${text(values, part.value)})`;
        return `${INDENT}(${text(values, part.name)}, ${value}),`;
      }),
      ']',
    );
    args.push('files=files');
  } else if (body?.mode === 'binary') {
    opensFile = body.path;
    args.push('data=payload');
  }
  args.push('timeout=30');
  const environment = usedVariables(values.variables, [...setup, ...args].join('\n'), local);
  const imports =
    environment.length > 0 ? ['import os', '', 'import requests'] : ['import requests'];
  if (signing?.kind === 'digest') {
    imports.push('from requests.auth import HTTPDigestAuth');
  }
  const sections: string[][] = [imports];
  if (environment.length > 0) {
    sections.push(
      environment.map(
        (variable) => `${variable.snake} = os.environ[${JSON.stringify(variable.env)}]`,
      ),
    );
  }
  sections.push(setup);

  const call = ['response = requests.request(', ...args.map((arg) => `${INDENT}${arg},`), ')'];
  sections.push(
    opensFile === null
      ? [...call, 'print(response.status_code)', 'print(response.text)']
      : [
          `with open(${JSON.stringify(opensFile)}, "rb") as payload:`,
          ...call.map((line) => `${INDENT}${line}`),
          'print(response.status_code)',
          'print(response.text)',
        ],
  );
  return sections.map((section) => section.join('\n')).join('\n\n');
}
