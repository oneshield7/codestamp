import { fileName } from '../body';
import { escaped, hex } from './escape';
import { sentHeaders, type CodeRequest } from './model';
import { encodedPairs, joined, lines, textSegments, urlSegments, type Segment } from './segments';
import { usedVariables, type CodeValues, type CodeVariable } from './values';

const INDENT = '    ';

const local = (variable: CodeVariable): string => variable.camel;

const SPECIAL = new Map([['"', '\\"']]);

function inner(text: string): string {
  return escaped(text, (code) => `\\u{${hex(code, 1)}}`, SPECIAL);
}

function string(segments: Segment[]): string {
  const parts = joined(segments);
  const [only] = parts;
  if (parts.length === 1 && only?.code !== undefined) {
    return only.code;
  }
  return `"${inside(parts)}"`;
}

/** The inside of a string literal, with each variable as an interpolation. */
function inside(segments: Segment[]): string {
  return joined(segments)
    .map((part) => (part.text !== undefined ? inner(part.text) : `\\(${part.code})`))
    .join('');
}

function text(values: CodeValues, value: string): string {
  return string(textSegments(values, value, local));
}

/** A multi-line literal keeps the body's lines; the closing quotes' indent is taken off each. */
function block(values: CodeValues, value: string): string {
  const content = joined(textSegments(values, value, local))
    .map((part) =>
      part.text !== undefined
        ? part.text.replace(/\\/g, '\\\\').replace(/"""/g, '\\"""')
        : `\\(${part.code})`,
    )
    .join('');
  const body = lines(content)
    .map((line) => (line === '' ? '' : `${INDENT}${line}`))
    .join('\n');
  return `"""\n${body}\n${INDENT}"""`;
}

export function swift(request: CodeRequest, values: CodeValues): string {
  const encode = (expression: string): string => `encoded(${expression})`;
  const setup = [
    `let url = URL(string: ${string(urlSegments(values, request.url, request.query, local, encode))})!`,
    'var request = URLRequest(url: url)',
    `request.httpMethod = ${JSON.stringify(request.method)}`,
  ];
  const signing = request.signing;
  if (request.basic) {
    const pair = text(values, `${request.basic.username}:${request.basic.password}`);
    setup.push(
      `let basic = Data(${pair}.utf8).base64EncodedString()`,
      'request.setValue("Basic \\(basic)", forHTTPHeaderField: "Authorization")',
    );
  } else if (signing?.kind === 'digest') {
    setup.push(
      '// Digest auth: answer the challenge in a URLSessionTaskDelegate with a URLCredential.',
    );
  } else if (signing?.kind === 'aws') {
    setup.push('// AWS Signature: sign this request with the AWS SDK for Swift.');
  }
  for (const header of sentHeaders(request)) {
    // addValue keeps headers that share a name; setValue would replace them.
    setup.push(
      `request.addValue(${text(values, header.value)}, forHTTPHeaderField: ${text(values, header.name)})`,
    );
  }

  const body = request.body;
  if (body?.mode === 'raw') {
    setup.push(
      `request.httpBody = Data(${body.text.includes('\n') ? block(values, body.text) : text(values, body.text)}.utf8)`,
    );
  } else if (body?.mode === 'urlencoded') {
    setup.push(
      `request.httpBody = Data(${string(encodedPairs(values, body.fields, local, encode))}.utf8)`,
    );
  } else if (body?.mode === 'formData') {
    // URLSession has no multipart builder, so the parts are written out with a boundary.
    setup.push(
      'let boundary = "codestamp-\\(UUID().uuidString)"',
      'request.setValue("multipart/form-data; boundary=\\(boundary)", forHTTPHeaderField: "Content-Type")',
      'var form = Data()',
    );
    for (const part of body.parts) {
      const name = inside(textSegments(values, part.name, local));
      const disposition = part.file
        ? `"--\\(boundary)\\r\\nContent-Disposition: form-data; name=\\"${name}\\"; filename=\\"${inner(fileName(part.value))}\\"\\r\\nContent-Type: application/octet-stream\\r\\n\\r\\n"`
        : `"--\\(boundary)\\r\\nContent-Disposition: form-data; name=\\"${name}\\"\\r\\n\\r\\n"`;
      setup.push(`form.append(Data(${disposition}.utf8))`);
      setup.push(
        part.file
          ? `form.append(try Data(contentsOf: URL(fileURLWithPath: "${inner(part.value)}")))`
          : `form.append(Data(${text(values, part.value)}.utf8))`,
      );
      setup.push('form.append(Data("\\r\\n".utf8))');
    }
    setup.push('form.append(Data("--\\(boundary)--\\r\\n".utf8))', 'request.httpBody = form');
  } else if (body?.mode === 'binary') {
    setup.push(
      `request.httpBody = try Data(contentsOf: URL(fileURLWithPath: "${inner(body.path)}"))`,
    );
  }

  const send = [
    'let (data, response) = try await URLSession.shared.data(for: request)',
    'print((response as! HTTPURLResponse).statusCode)',
    'print(String(decoding: data, as: UTF8.self))',
  ];
  const header = [
    'import Foundation',
    '#if canImport(FoundationNetworking)',
    'import FoundationNetworking',
    '#endif',
  ];
  const sections = [header];
  const environment = usedVariables(values.variables, setup.join('\n'), local);
  if (environment.length > 0) {
    sections.push([
      'let environment = ProcessInfo.processInfo.environment',
      ...environment.map(
        (variable) => `let ${variable.camel} = environment[${JSON.stringify(variable.env)}] ?? ""`,
      ),
    ]);
  }
  if (setup.some((line) => line.includes('encoded('))) {
    sections.push([
      '/// Percent-encodes a query or form value, leaving only unreserved characters as they are.',
      'func encoded(_ value: String) -> String {',
      `${INDENT}let unreserved = CharacterSet(charactersIn: "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-._~")`,
      `${INDENT}return value.addingPercentEncoding(withAllowedCharacters: unreserved) ?? value`,
      '}',
    ]);
  }
  sections.push(setup, send);
  return sections.map((section) => section.join('\n')).join('\n\n');
}
