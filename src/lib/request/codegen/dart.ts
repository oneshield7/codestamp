import { hasDuplicateNames, mergedHeaders, sentHeaders, type CodeRequest } from './model';
import { encodedPairs, joined, lines, textSegments, urlSegments, type Segment } from './segments';
import { usedVariables, type CodeValues, type CodeVariable } from './values';

const INDENT = '  ';

const local = (variable: CodeVariable): string => variable.camel;

/** Single quotes, with `$` escaped since it starts an interpolation. */
function plain(text: string): string {
  return text
    .replace(/[\\'$]/g, '\\$&')
    .replace(/\n/g, '\\n')
    .replace(/\r/g, '\\r')
    .replace(/\t/g, '\\t');
}

function string(segments: Segment[]): string {
  const parts = joined(segments);
  const [only] = parts;
  if (parts.length === 1 && only?.code !== undefined) {
    return only.code;
  }
  const inner = parts
    .map((part) => (part.text !== undefined ? plain(part.text) : `\${${part.code}}`))
    .join('');
  return `'${inner}'`;
}

function text(values: CodeValues, value: string): string {
  return string(textSegments(values, value, local));
}

/** A triple-quoted string keeps the body's lines; its first line break is not part of the text. */
function block(values: CodeValues, value: string): string {
  const inner = joined(textSegments(values, value, local))
    .map((part) =>
      part.text !== undefined
        ? part.text.replace(/[\\$]/g, '\\$&').replace(/'''/g, "\\'\\'\\'").replace(/'$/, "\\'")
        : `\${${part.code}}`,
    )
    .join('');
  return `'''\n${lines(inner).join('\n')}'''`;
}

export function dart(request: CodeRequest, values: CodeValues): string {
  const imports = new Set(["import 'package:http/http.dart' as http;"]);
  const encode = (expression: string): string => `Uri.encodeQueryComponent(${expression})`;
  const setup: string[] = [
    `${INDENT}final url = Uri.parse(${string(urlSegments(values, request.url, request.query, local, encode))});`,
  ];
  const body = request.body;
  const signing = request.signing;
  const multipart = body?.mode === 'formData';
  setup.push(
    multipart
      ? `${INDENT}final request = http.MultipartRequest('${request.method}', url);`
      : `${INDENT}final request = http.Request('${request.method}', url);`,
  );

  const headers = mergedHeaders(sentHeaders(request)).map((header): [string, string] => [
    text(values, header.name),
    text(values, header.value),
  ]);
  if (request.basic) {
    imports.add("import 'dart:convert';");
    const pair = text(values, `${request.basic.username}:${request.basic.password}`);
    headers.unshift(["'Authorization'", `'Basic \${base64Encode(utf8.encode(${pair}))}'`]);
  } else if (signing?.kind === 'digest') {
    setup.push(`${INDENT}// Digest auth: package:http cannot answer the server's challenge.`);
  } else if (signing?.kind === 'aws') {
    setup.push(`${INDENT}// AWS Signature: sign this request with the AWS SDK for Dart.`);
  }
  if (headers.length > 0) {
    setup.push(
      `${INDENT}request.headers.addAll({`,
      ...headers.map(([name, value]) => `${INDENT}${INDENT}${name}: ${value},`),
      `${INDENT}});`,
    );
  }

  if (body?.mode === 'raw') {
    setup.push(
      `${INDENT}request.body = ${body.text.includes('\n') ? block(values, body.text) : text(values, body.text)};`,
    );
  } else if (body?.mode === 'urlencoded') {
    // A map keeps one value per name, so repeated names are sent as an encoded string.
    if (hasDuplicateNames(body.fields)) {
      setup.push(
        `${INDENT}request.body = ${string(encodedPairs(values, body.fields, local, encode))};`,
      );
    } else {
      setup.push(
        `${INDENT}request.bodyFields = {`,
        ...body.fields.map(
          (field) => `${INDENT}${INDENT}${text(values, field.name)}: ${text(values, field.value)},`,
        ),
        `${INDENT}};`,
      );
    }
  } else if (body?.mode === 'formData') {
    for (const part of body.parts) {
      setup.push(
        part.file
          ? `${INDENT}request.files.add(await http.MultipartFile.fromPath(${text(values, part.name)}, '${plain(part.value)}'));`
          : `${INDENT}request.fields[${text(values, part.name)}] = ${text(values, part.value)};`,
      );
    }
  } else if (body?.mode === 'binary') {
    imports.add("import 'dart:io';");
    setup.push(`${INDENT}request.bodyBytes = await File('${plain(body.path)}').readAsBytes();`);
  }

  const send = [
    `${INDENT}final response = await http.Response.fromStream(await request.send());`,
    `${INDENT}print(response.statusCode);`,
    `${INDENT}print(response.body);`,
  ];
  const sections = [setup, send];
  const environment = usedVariables(values.variables, setup.join('\n'), local);
  if (environment.length > 0) {
    imports.add("import 'dart:io';");
    sections.unshift(
      environment.map(
        (variable) =>
          `${INDENT}final ${variable.camel} = Platform.environment['${variable.env}']!;`,
      ),
    );
  }
  // dart: imports come before package: imports, as the Dart style guide orders them.
  const dartImports = [...imports].filter((line) => line.includes("'dart:")).sort();
  const packageImports = [...imports].filter((line) => line.includes("'package:")).sort();
  return [
    ...dartImports,
    ...(dartImports.length > 0 ? [''] : []),
    ...packageImports,
    '',
    'Future<void> main() async {',
    sections.map((section) => section.join('\n')).join('\n\n'),
    '}',
  ].join('\n');
}
