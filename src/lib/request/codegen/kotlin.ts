import { fileName } from '../body';
import { escaped, hex } from './escape';
import { headerValue, mergedHeaders, sentHeaders, type CodeRequest } from './model';
import { joined, lines, textSegments, type Segment } from './segments';
import { usedVariables, type CodeValues, type CodeVariable } from './values';

const I1 = '    ';
const I2 = I1.repeat(2);

const local = (variable: CodeVariable): string => variable.camel;

/** `$` starts a template, so it is escaped like the quote. */
const SPECIAL = new Map([
  ['"', '\\"'],
  ['$', '\\$'],
]);

function quoted(text: string): string {
  return `"${escaped(text, (code) => `\\u${hex(code, 4)}`, SPECIAL)}"`;
}

function string(segments: Segment[]): string {
  const parts = joined(segments);
  const [only] = parts;
  if (parts.length === 1 && only?.code !== undefined) {
    return only.code;
  }
  const inner = parts
    .map((part) => (part.text !== undefined ? quoted(part.text).slice(1, -1) : `\${${part.code}}`))
    .join('');
  return `"${inner}"`;
}

function text(values: CodeValues, value: string): string {
  return string(textSegments(values, value, local));
}

/**
 * A raw string keeps the body's lines; `trimIndent` takes off the indent. Raw strings have no
 * escapes, so `$` and `"""` are written as templates. A body that starts or ends with a blank
 * line would lose it to `trimIndent`, so it stays a one-line string.
 */
function block(values: CodeValues, value: string): string {
  const all = lines(value);
  if ((all[0] ?? '').trim() === '' || (all.at(-1) ?? '').trim() === '') {
    return text(values, value);
  }
  const inner = joined(textSegments(values, value, local))
    .map((part) =>
      part.text !== undefined
        ? // Replacer functions, since `$'` in a replacement string means "the text after".
          part.text.replace(/\$/g, () => "${'$'}").replace(/"""/g, () => '${"\\"\\"\\""}')
        : `\${${part.code}}`,
    )
    .join('');
  const body = lines(inner)
    .map((line) => (line === '' ? '' : `${I2}${line}`))
    .join('\n');
  return `"""\n${body}\n${I1}""".trimIndent()`;
}

export function kotlin(request: CodeRequest, values: CodeValues): string {
  const imports = new Set(['okhttp3.OkHttpClient', 'okhttp3.Request']);
  const setup: string[] = [];

  let url = text(values, request.url);
  if (request.query.length > 0) {
    imports.add('okhttp3.HttpUrl.Companion.toHttpUrl');
    setup.push(
      `${I1}val url = ${url}.toHttpUrl().newBuilder()`,
      ...request.query.map(
        (pair) =>
          `${I2}.addQueryParameter(${text(values, pair.name)}, ${text(values, pair.value)})`,
      ),
      `${I2}.build()`,
    );
    url = 'url';
  }

  const headers = mergedHeaders(sentHeaders(request));
  const contentType = headerValue(headers, 'content-type');
  const mediaType = (): string => {
    if (contentType === undefined) {
      return '';
    }
    imports.add('okhttp3.MediaType.Companion.toMediaType');
    return `${text(values, contentType)}.toMediaType()`;
  };
  const body = request.body;
  let requestBody = 'null';
  if (body?.mode === 'raw') {
    imports.add('okhttp3.RequestBody.Companion.toRequestBody');
    setup.push(
      `${I1}val body = ${body.text.includes('\n') ? block(values, body.text) : text(values, body.text)}`,
    );
    requestBody = `body.toRequestBody(${mediaType()})`;
  } else if (body?.mode === 'urlencoded') {
    imports.add('okhttp3.FormBody');
    setup.push(
      `${I1}val form = FormBody.Builder()`,
      ...body.fields.map(
        (field) => `${I2}.add(${text(values, field.name)}, ${text(values, field.value)})`,
      ),
      `${I2}.build()`,
    );
    requestBody = 'form';
  } else if (body?.mode === 'formData') {
    imports.add('okhttp3.MultipartBody');
    setup.push(`${I1}val form = MultipartBody.Builder()`, `${I2}.setType(MultipartBody.FORM)`);
    for (const part of body.parts) {
      if (part.file) {
        imports.add('java.io.File').add('okhttp3.RequestBody.Companion.asRequestBody');
        setup.push(
          `${I2}.addFormDataPart(${text(values, part.name)}, ${quoted(fileName(part.value))}, File(${quoted(part.value)}).asRequestBody())`,
        );
      } else {
        setup.push(
          `${I2}.addFormDataPart(${text(values, part.name)}, ${text(values, part.value)})`,
        );
      }
    }
    setup.push(`${I2}.build()`);
    requestBody = 'form';
  } else if (body?.mode === 'binary') {
    imports.add('java.io.File').add('okhttp3.RequestBody.Companion.asRequestBody');
    requestBody = `File(${quoted(body.path)}).asRequestBody(${mediaType()})`;
  } else if (!['GET', 'HEAD', 'DELETE', 'OPTIONS'].includes(request.method)) {
    // OkHttp insists on a body for POST, PUT and PATCH, even an empty one.
    imports.add('okhttp3.RequestBody.Companion.toRequestBody');
    requestBody = 'ByteArray(0).toRequestBody()';
  }

  const builder = [`${I1}val request = Request.Builder()`, `${I2}.url(${url})`];
  const signing = request.signing;
  if (request.basic) {
    imports.add('okhttp3.Credentials');
    builder.push(
      `${I2}.header("Authorization", Credentials.basic(${text(values, request.basic.username)}, ${text(values, request.basic.password)}, Charsets.UTF_8))`,
    );
  } else if (signing?.kind === 'digest') {
    setup.push(
      `${I1}// Digest auth: OkHttp cannot answer the server's challenge; use okhttp-digest.`,
    );
  } else if (signing?.kind === 'aws') {
    setup.push(`${I1}// AWS Signature: sign this request with the AWS SDK for Kotlin.`);
  }
  // The body carries its own content type, so a Content-Type header would be sent twice.
  for (const header of headers) {
    if (header.name.toLowerCase() !== 'content-type' || body === null) {
      builder.push(`${I2}.addHeader(${text(values, header.name)}, ${text(values, header.value)})`);
    }
  }
  builder.push(`${I2}.method(${JSON.stringify(request.method)}, ${requestBody})`, `${I2}.build()`);

  const send = [
    `${I1}OkHttpClient().newCall(request).execute().use { response ->`,
    `${I2}println(response.code)`,
    `${I2}println(response.body.string())`,
    `${I1}}`,
  ];
  const sections = [setup, builder, send].filter((section) => section.length > 0);
  const environment = usedVariables(values.variables, [...setup, ...builder].join('\n'), local);
  if (environment.length > 0) {
    sections.unshift(
      environment.map(
        (variable) => `${I1}val ${variable.camel} = System.getenv(${JSON.stringify(variable.env)})`,
      ),
    );
  }
  return [
    `// OkHttp 5 (com.squareup.okhttp3:okhttp).`,
    ...[...imports].sort().map((name) => `import ${name}`),
    '',
    'fun main() {',
    sections.map((section) => section.join('\n')).join('\n\n'),
    '}',
  ].join('\n');
}
