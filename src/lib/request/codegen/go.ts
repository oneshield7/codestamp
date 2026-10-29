import { fileName } from '../body';
import type { NameValue } from '../../api/http';
import { sentHeaders, type CodeRequest } from './model';
import { encodedPairs, joined, textSegments, type Segment } from './segments';
import { usedVariables, type CodeValues, type CodeVariable } from './values';

const T = '\t';

const local = (variable: CodeVariable): string => variable.camel;

/** A raw string keeps lines as they are; it cannot hold a backquote or a carriage return. */
function literal(text: string): string {
  return text.includes('\n') && !/[`\r]/.test(text) ? `\`${text}\`` : JSON.stringify(text);
}

function string(segments: Segment[]): string {
  const parts = joined(segments);
  if (parts.length === 0) {
    return '""';
  }
  return parts
    .map((part) => (part.text !== undefined ? literal(part.text) : part.code))
    .join(' + ');
}

function text(values: CodeValues, value: string): string {
  return string(textSegments(values, value, local));
}

function check(indent: string): string[] {
  return [`${indent}if err != nil {`, `${indent}${T}panic(err)`, `${indent}}`];
}

export function goCode(request: CodeRequest, values: CodeValues): string {
  const imports = new Set(['fmt', 'io', 'net/http']);
  const encode = (expression: string): string => {
    imports.add('net/url');
    return `url.QueryEscape(${expression})`;
  };
  // url.Values sorts by name when it encodes, so the pairs are joined in their own order.
  const pairList = (name: string, pairs: NameValue[]): string[] => {
    imports.add('strings');
    return [
      `${T}${name} := strings.Join([]string{`,
      ...pairs.map((pair) => `${T}${T}${string(encodedPairs(values, [pair], local, encode))},`),
      `${T}}, "&")`,
    ];
  };
  const setup: string[] = [];
  let url = text(values, request.url);
  if (request.query.length > 0) {
    setup.push(...pairList('query', request.query));
    url = `${url} + ${JSON.stringify(request.url.includes('?') ? '&' : '?')} + query`;
  }

  const body = request.body;
  let reader = 'nil';
  let contentType: string | null = null;
  if (body?.mode === 'raw') {
    imports.add('strings');
    setup.push(`${T}body := strings.NewReader(${text(values, body.text)})`);
    reader = 'body';
  } else if (body?.mode === 'urlencoded') {
    setup.push(...pairList('form', body.fields), `${T}body := strings.NewReader(form)`);
    reader = 'body';
  } else if (body?.mode === 'formData') {
    imports.add('bytes').add('mime/multipart');
    setup.push(`${T}var body bytes.Buffer`, `${T}writer := multipart.NewWriter(&body)`);
    body.parts.forEach((part, index) => {
      if (!part.file) {
        setup.push(
          `${T}if err := writer.WriteField(${text(values, part.name)}, ${text(values, part.value)}); err != nil {`,
          `${T}${T}panic(err)`,
          `${T}}`,
        );
        return;
      }
      imports.add('os');
      const file = `file${String(index + 1)}`;
      const part_ = `part${String(index + 1)}`;
      setup.push(
        `${T}${file}, err := os.Open(${JSON.stringify(part.value)})`,
        ...check(T),
        `${T}defer ${file}.Close()`,
        `${T}${part_}, err := writer.CreateFormFile(${text(values, part.name)}, ${JSON.stringify(fileName(part.value))})`,
        ...check(T),
        `${T}if _, err := io.Copy(${part_}, ${file}); err != nil {`,
        `${T}${T}panic(err)`,
        `${T}}`,
      );
    });
    setup.push(`${T}if err := writer.Close(); err != nil {`, `${T}${T}panic(err)`, `${T}}`);
    reader = '&body';
    contentType = 'writer.FormDataContentType()';
  } else if (body?.mode === 'binary') {
    imports.add('os');
    setup.push(
      `${T}body, err := os.Open(${JSON.stringify(body.path)})`,
      ...check(T),
      `${T}defer body.Close()`,
    );
    reader = 'body';
  }
  setup.push(
    `${T}req, err := http.NewRequest(${JSON.stringify(request.method)}, ${url}, ${reader})`,
    ...check(T),
  );

  const signing = request.signing;
  if (request.basic) {
    setup.push(
      `${T}req.SetBasicAuth(${text(values, request.basic.username)}, ${text(values, request.basic.password)})`,
    );
  } else if (signing?.kind === 'digest') {
    setup.push(
      `${T}// Digest auth: net/http cannot answer the server's challenge; use a Digest client.`,
    );
  } else if (signing?.kind === 'aws') {
    setup.push(`${T}// AWS Signature: sign this request with aws-sdk-go-v2 (aws/signer/v4).`);
  }
  for (const header of sentHeaders(request)) {
    setup.push(`${T}req.Header.Add(${text(values, header.name)}, ${text(values, header.value)})`);
  }
  if (contentType) {
    setup.push(`${T}req.Header.Set("Content-Type", ${contentType})`);
  }
  const send = [
    `${T}resp, err := http.DefaultClient.Do(req)`,
    ...check(T),
    `${T}defer resp.Body.Close()`,
    '',
    `${T}data, err := io.ReadAll(resp.Body)`,
    ...check(T),
    `${T}fmt.Println(resp.StatusCode)`,
    `${T}fmt.Println(string(data))`,
  ];
  const sections = [setup, send];
  const environment = usedVariables(values.variables, setup.join('\n'), local);
  if (environment.length > 0) {
    imports.add('os');
    sections.unshift(
      environment.map(
        (variable) => `${T}${variable.camel} := os.Getenv(${JSON.stringify(variable.env)})`,
      ),
    );
  }

  const sorted = [...imports].sort();
  return [
    'package main',
    '',
    'import (',
    ...sorted.map((name) => `${T}${JSON.stringify(name)}`),
    ')',
    '',
    'func main() {',
    sections.map((section) => section.join('\n')).join('\n\n'),
    '}',
  ].join('\n');
}
