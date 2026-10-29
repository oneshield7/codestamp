import { mergedHeaders, sentHeaders, titleCase, type CodeRequest } from './model';
import { joined, lines, textSegments, type Segment } from './segments';
import { usedVariables, type CodeValues, type CodeVariable } from './values';

const INDENT = '  ';

const local = (variable: CodeVariable): string => variable.snake;

function single(text: string): string {
  return `'${text.replace(/[\\']/g, '\\$&')}'`;
}

/** Double quotes only around `#{…}`; `#` is escaped where it would start an interpolation. */
function string(segments: Segment[]): string {
  const parts = joined(segments);
  const [only] = parts;
  if (parts.length === 1 && only?.code !== undefined) {
    return only.code;
  }
  if (parts.every((part) => part.text !== undefined)) {
    return single(parts.map((part) => part.text).join(''));
  }
  const inner = parts
    .map((part) =>
      part.text !== undefined
        ? JSON.stringify(part.text)
            .slice(1, -1)
            .replace(/#(?=[{@$])/g, '\\#')
        : `#{${part.code}}`,
    )
    .join('');
  return `"${inner}"`;
}

function text(values: CodeValues, value: string): string {
  return string(textSegments(values, value, local));
}

function pairs(values: CodeValues, items: { name: string; value: string }[]): string[] {
  return items.map((item) => `${INDENT}[${text(values, item.name)}, ${text(values, item.value)}],`);
}

/**
 * A squiggly heredoc keeps the body's lines and takes off their indent; `chomp` drops the line
 * break it adds at the end. The quoted form reads the text as it is.
 */
function block(values: CodeValues, value: string): string {
  const parts = joined(textSegments(values, value, local));
  const plain = parts.every((part) => part.text !== undefined);
  let marker = 'BODY';
  while (lines(value).some((line) => line.trim() === marker)) {
    marker = `${marker}_END`;
  }
  const content = parts
    .map((part) =>
      part.text === undefined
        ? `#{${part.code}}`
        : plain
          ? part.text
          : part.text.replace(/\\/g, '\\\\').replace(/#(?=[{@$])/g, '\\#'),
    )
    .join('');
  const body = lines(content)
    .map((line) => (line === '' ? '' : `${INDENT}${line}`))
    .join('\n');
  return `<<~${plain ? `'${marker}'` : marker}.chomp\n${body}\n${marker}`;
}

const CLASSES = new Map([
  ['GET', 'Get'],
  ['POST', 'Post'],
  ['PUT', 'Put'],
  ['PATCH', 'Patch'],
  ['DELETE', 'Delete'],
  ['HEAD', 'Head'],
  ['OPTIONS', 'Options'],
]);

export function ruby(request: CodeRequest, values: CodeValues): string {
  const setup: string[] = [`uri = URI(${text(values, request.url)})`];
  if (request.query.length > 0) {
    // Appended to any query the URL already has, in order and with repeated names kept.
    setup.push(
      'uri.query = [uri.query, URI.encode_www_form([',
      ...pairs(values, request.query),
      "])].compact.join('&')",
    );
  }
  setup.push(
    '',
    `request = Net::HTTP::${CLASSES.get(request.method) ?? titleCase(request.method)}.new(uri)`,
  );
  const signing = request.signing;
  if (request.basic) {
    setup.push(
      `request.basic_auth(${text(values, request.basic.username)}, ${text(values, request.basic.password)})`,
    );
  } else if (signing?.kind === 'digest') {
    setup.push(
      "# Digest auth: Net::HTTP cannot answer the server's challenge; use the net-http-digest_auth gem.",
    );
  } else if (signing?.kind === 'aws') {
    setup.push('# AWS Signature: sign this request with the aws-sigv4 gem.');
  }
  for (const header of mergedHeaders(sentHeaders(request))) {
    setup.push(`request[${text(values, header.name)}] = ${text(values, header.value)}`);
  }

  const body = request.body;
  if (body?.mode === 'raw') {
    setup.push(
      `request.body = ${body.text.includes('\n') ? block(values, body.text) : text(values, body.text)}`,
    );
  } else if (body?.mode === 'urlencoded') {
    setup.push('request.body = URI.encode_www_form([', ...pairs(values, body.fields), '])');
  } else if (body?.mode === 'formData') {
    setup.push(
      'request.set_form([',
      ...body.parts.map((part) =>
        part.file
          ? `${INDENT}[${text(values, part.name)}, File.open(${single(part.value)})],`
          : `${INDENT}[${text(values, part.name)}, ${text(values, part.value)}],`,
      ),
      "], 'multipart/form-data')",
    );
  } else if (body?.mode === 'binary') {
    setup.push(`request.body = File.binread(${single(body.path)})`);
  }

  const send = [
    "response = Net::HTTP.start(uri.hostname, uri.port, use_ssl: uri.scheme == 'https') do |http|",
    `${INDENT}http.request(request)`,
    'end',
    'puts response.code',
    'puts response.body',
  ];
  const sections = [["require 'net/http'", "require 'uri'"]];
  const environment = usedVariables(values.variables, setup.join('\n'), local);
  if (environment.length > 0) {
    sections.push(
      environment.map((variable) => `${variable.snake} = ENV.fetch(${single(variable.env)})`),
    );
  }
  sections.push(setup, send);
  return sections.map((section) => section.join('\n')).join('\n\n');
}
