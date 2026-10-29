import { fileName } from '../body';
import { sentHeaders, type CodeRequest } from './model';
import { encodedPairs, joined, lines, textSegments, urlSegments, type Segment } from './segments';
import { usedVariables, type CodeValues, type CodeVariable } from './values';

const I1 = '    ';
const I2 = I1.repeat(2);
const I3 = I1.repeat(3);

/** HttpClient sets these itself and refuses them in a request. */
const RESTRICTED = new Set(['connection', 'content-length', 'expect', 'host', 'upgrade']);

const local = (variable: CodeVariable): string => variable.camel;

/** A JSON string literal is also a Java one. */
function string(segments: Segment[]): string {
  const parts = joined(segments);
  if (parts.length === 0) {
    return '""';
  }
  return parts
    .map((part) => (part.text !== undefined ? JSON.stringify(part.text) : part.code))
    .join(' + ');
}

function text(values: CodeValues, value: string): string {
  return string(textSegments(values, value, local));
}

/**
 * A text block keeps the body's lines (Java 15 or later); with variables it is filled in by
 * `formatted`. The closing quotes sit on their own line, so the indent is taken off every line.
 */
function block(values: CodeValues, value: string): string {
  const parts = joined(textSegments(values, value, local));
  const plain = parts.every((part) => part.text !== undefined);
  const inner = parts
    .map((part) => {
      if (part.text === undefined) {
        return '%s';
      }
      const escaped = part.text.replace(/\\/g, '\\\\').replace(/"""/g, '\\"""');
      return plain ? escaped : escaped.replace(/%/g, '%%');
    })
    .join('');
  const body = lines(inner)
    // Trailing spaces are stripped from text blocks unless the last one is escaped.
    .map((line) => line.replace(/[ \t]$/, (space) => (space === ' ' ? '\\s' : '\\t')))
    .map((line, index, all) => `${I3}${line}${index === all.length - 1 ? '\\' : ''}`)
    .join('\n');
  const codes = parts.flatMap((part) => (part.code === undefined ? [] : [part.code]));
  return `"""\n${body}\n${I3}"""${plain ? '' : `.formatted(${codes.join(', ')})`}`;
}

export function java(request: CodeRequest, values: CodeValues): string {
  const imports = new Set([
    'java.net.URI',
    'java.net.http.HttpClient',
    'java.net.http.HttpRequest',
    'java.net.http.HttpResponse',
  ]);
  const charset = (): string => {
    imports.add('java.nio.charset.StandardCharsets');
    return 'StandardCharsets.UTF_8';
  };
  const encode = (expression: string): string => {
    imports.add('java.net.URLEncoder');
    return `URLEncoder.encode(${expression}, ${charset()})`;
  };

  const setup: string[] = [];
  setup.push(
    `${I2}String url = ${string(urlSegments(values, request.url, request.query, local, encode))};`,
  );
  const body = request.body;
  let publisher = 'HttpRequest.BodyPublishers.noBody()';
  let contentType: string | null = null;
  if (body?.mode === 'raw') {
    setup.push(
      `${I2}String body = ${body.text.includes('\n') ? block(values, body.text) : text(values, body.text)};`,
    );
    publisher = 'HttpRequest.BodyPublishers.ofString(body)';
  } else if (body?.mode === 'urlencoded') {
    setup.push(`${I2}String body = ${string(encodedPairs(values, body.fields, local, encode))};`);
    publisher = 'HttpRequest.BodyPublishers.ofString(body)';
  } else if (body?.mode === 'formData') {
    // HttpClient has no multipart builder, so the parts are written out with a boundary.
    imports.add('java.io.ByteArrayOutputStream').add('java.util.UUID');
    setup.push(
      `${I2}String boundary = "codestamp-" + UUID.randomUUID();`,
      `${I2}ByteArrayOutputStream form = new ByteArrayOutputStream();`,
    );
    for (const part of body.parts) {
      const disposition = part.file
        ? `"--" + boundary + "\\r\\nContent-Disposition: form-data; name=\\"" + ${text(values, part.name)} + "\\"; filename=\\"" + ${JSON.stringify(fileName(part.value))} + "\\"\\r\\nContent-Type: application/octet-stream\\r\\n\\r\\n"`
        : `"--" + boundary + "\\r\\nContent-Disposition: form-data; name=\\"" + ${text(values, part.name)} + "\\"\\r\\n\\r\\n"`;
      setup.push(`${I2}form.write((${disposition}).getBytes(${charset()}));`);
      if (part.file) {
        imports.add('java.nio.file.Files').add('java.nio.file.Path');
        setup.push(`${I2}form.write(Files.readAllBytes(Path.of(${JSON.stringify(part.value)})));`);
      } else {
        setup.push(`${I2}form.write((${text(values, part.value)}).getBytes(${charset()}));`);
      }
      setup.push(`${I2}form.write("\\r\\n".getBytes(${charset()}));`);
    }
    setup.push(`${I2}form.write(("--" + boundary + "--\\r\\n").getBytes(${charset()}));`);
    publisher = 'HttpRequest.BodyPublishers.ofByteArray(form.toByteArray())';
    contentType = '"multipart/form-data; boundary=" + boundary';
  } else if (body?.mode === 'binary') {
    imports.add('java.nio.file.Path');
    publisher = `HttpRequest.BodyPublishers.ofFile(Path.of(${JSON.stringify(body.path)}))`;
  }

  const builder = [`${I2}HttpRequest request = HttpRequest.newBuilder(URI.create(url))`];
  const signing = request.signing;
  if (request.basic) {
    imports.add('java.util.Base64');
    const pair = text(values, `${request.basic.username}:${request.basic.password}`);
    setup.push(
      `${I2}String basic = Base64.getEncoder().encodeToString((${pair}).getBytes(${charset()}));`,
    );
    builder.push(`${I3}.header("Authorization", "Basic " + basic)`);
  } else if (signing?.kind === 'digest') {
    setup.push(
      `${I2}// Digest auth: HttpClient cannot answer the server's challenge; use a Digest client.`,
    );
  } else if (signing?.kind === 'aws') {
    setup.push(`${I2}// AWS Signature: sign this request with the AWS SDK for Java (Aws4Signer).`);
  }
  for (const header of sentHeaders(request)) {
    if (RESTRICTED.has(header.name.toLowerCase())) {
      builder.push(`${I3}// ${header.name} is set by HttpClient itself.`);
    } else {
      builder.push(`${I3}.header(${text(values, header.name)}, ${text(values, header.value)})`);
    }
  }
  if (contentType) {
    builder.push(`${I3}.header("Content-Type", ${contentType})`);
  }
  builder.push(`${I3}.method(${JSON.stringify(request.method)}, ${publisher})`, `${I3}.build();`);

  const send = [
    `${I2}HttpResponse<String> response = HttpClient.newHttpClient()`,
    `${I3}.send(request, HttpResponse.BodyHandlers.ofString());`,
    `${I2}System.out.println(response.statusCode());`,
    `${I2}System.out.println(response.body());`,
  ];
  const sections = [setup, builder, send];
  const environment = usedVariables(values.variables, [...setup, ...builder].join('\n'), local);
  if (environment.length > 0) {
    sections.unshift(
      environment.map(
        (variable) =>
          `${I2}String ${variable.camel} = System.getenv(${JSON.stringify(variable.env)});`,
      ),
    );
  }
  return [
    ...[...imports].sort().map((name) => `import ${name};`),
    '',
    'public class Main {',
    `${I1}public static void main(String[] args) throws Exception {`,
    sections.map((section) => section.join('\n')).join('\n\n'),
    `${I1}}`,
    '}',
  ].join('\n');
}
