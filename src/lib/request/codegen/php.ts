import { fileName } from '../body';
import { hasDuplicateNames, sentHeaders, type CodeRequest } from './model';
import { encodedPairs, joined, lines, textSegments, type Segment } from './segments';
import { usedVariables, type CodeValues, type CodeVariable } from './values';

const INDENT = '    ';

const local = (variable: CodeVariable): string => `$${variable.camel}`;

function single(text: string): string {
  return `'${text.replace(/[\\']/g, '\\$&')}'`;
}

/** Single-quoted text joined to variables with `.`. */
function string(segments: Segment[]): string {
  const parts = joined(segments);
  if (parts.length === 0) {
    return "''";
  }
  return parts.map((part) => (part.text !== undefined ? single(part.text) : part.code)).join(' . ');
}

function text(values: CodeValues, value: string): string {
  return string(textSegments(values, value, local));
}

/**
 * A nowdoc keeps the body's lines; with variables it becomes a heredoc around `{$name}`. The
 * closing marker's indent is taken off every line (PHP 7.3 or later).
 */
function block(values: CodeValues, value: string, indent: string): string {
  const parts = joined(textSegments(values, value, local));
  const plain = parts.every((part) => part.text !== undefined);
  let marker = 'BODY';
  while (lines(value).some((line) => line.trim().startsWith(marker))) {
    marker = `${marker}_END`;
  }
  const inner = parts
    .map((part) =>
      part.text === undefined
        ? `{${part.code}}`
        : plain
          ? part.text
          : part.text.replace(/[\\$]/g, '\\$&'),
    )
    .join('');
  const body = lines(inner)
    .map((line) => (line === '' ? '' : `${indent}${line}`))
    .join('\n');
  return `<<<${plain ? `'${marker}'` : marker}\n${body}\n${indent}${marker}`;
}

function entries(pairs: [string, string][], depth: number): string[] {
  return pairs.map(([key, value]) => `${INDENT.repeat(depth)}${key} => ${value},`);
}

export function php(request: CodeRequest, values: CodeValues): string {
  const options: string[] = [];
  const encode = (expression: string): string => `rawurlencode(${expression})`;
  if (request.query.length > 0) {
    // An array keeps one value per name, so repeated names are sent as an encoded string.
    options.push(
      ...(hasDuplicateNames(request.query)
        ? [`${INDENT}'query' => ${string(encodedPairs(values, request.query, local, encode))},`]
        : [
            `${INDENT}'query' => [`,
            ...entries(
              request.query.map((pair) => [text(values, pair.name), text(values, pair.value)]),
              2,
            ),
            `${INDENT}],`,
          ]),
    );
  }
  const signing = request.signing;
  if (request.basic) {
    options.push(
      `${INDENT}'auth' => [${text(values, request.basic.username)}, ${text(values, request.basic.password)}],`,
    );
  } else if (signing?.kind === 'digest') {
    options.push(
      `${INDENT}'auth' => [${text(values, signing.username)}, ${text(values, signing.password)}, 'digest'],`,
    );
  }
  const headers = sentHeaders(request);
  if (headers.length > 0) {
    const grouped = new Map<string, string[]>();
    for (const header of headers) {
      const key = text(values, header.name);
      grouped.set(key, [...(grouped.get(key) ?? []), text(values, header.value)]);
    }
    options.push(
      `${INDENT}'headers' => [`,
      ...entries(
        [...grouped].map(([key, list]) => [
          key,
          list.length === 1 ? (list[0] ?? "''") : `[${list.join(', ')}]`,
        ]),
        2,
      ),
      `${INDENT}],`,
    );
  }
  const body = request.body;
  if (body?.mode === 'raw') {
    options.push(
      body.text.includes('\n')
        ? `${INDENT}'body' => ${block(values, body.text, INDENT.repeat(2))},`
        : `${INDENT}'body' => ${text(values, body.text)},`,
    );
  } else if (body?.mode === 'urlencoded') {
    options.push(
      ...(hasDuplicateNames(body.fields)
        ? [`${INDENT}'body' => ${string(encodedPairs(values, body.fields, local, encode))},`]
        : [
            `${INDENT}'form_params' => [`,
            ...entries(
              body.fields.map((field) => [text(values, field.name), text(values, field.value)]),
              2,
            ),
            `${INDENT}],`,
          ]),
    );
  } else if (body?.mode === 'formData') {
    options.push(`${INDENT}'multipart' => [`);
    for (const part of body.parts) {
      const fields: [string, string][] = part.file
        ? [
            ["'name'", text(values, part.name)],
            ["'contents'", `fopen(${single(part.value)}, 'r')`],
            ["'filename'", single(fileName(part.value))],
          ]
        : [
            ["'name'", text(values, part.name)],
            ["'contents'", text(values, part.value)],
          ];
      options.push(`${INDENT.repeat(2)}[`, ...entries(fields, 3), `${INDENT.repeat(2)}],`);
    }
    options.push(`${INDENT}],`);
  } else if (body?.mode === 'binary') {
    options.push(`${INDENT}'body' => fopen(${single(body.path)}, 'r'),`);
  }
  // Error statuses are returned like any other, so the code can print them.
  options.push(`${INDENT}'http_errors' => false,`);

  const send: string[] = [];
  if (signing?.kind === 'aws') {
    send.push('// AWS Signature: sign this request with the AWS SDK for PHP (SignatureV4).');
  }
  send.push(
    '$client = new Client();',
    `$response = $client->request(${single(request.method)}, ${text(values, request.url)}, [`,
    ...options,
    ']);',
    '',
    'echo $response->getStatusCode(), PHP_EOL;',
    'echo $response->getBody(), PHP_EOL;',
  );
  const sections: string[][] = [
    ['<?php'],
    ["require __DIR__ . '/vendor/autoload.php';", '', 'use GuzzleHttp\\Client;'],
  ];
  const environment = usedVariables(values.variables, send.join('\n'), (item) => item.camel);
  if (environment.length > 0) {
    sections.push(
      environment.map((variable) => `$${variable.camel} = getenv(${single(variable.env)});`),
    );
  }
  sections.push(send);
  return sections.map((section) => section.join('\n')).join('\n\n');
}
