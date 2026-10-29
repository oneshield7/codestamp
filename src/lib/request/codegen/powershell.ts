import { fileName } from '../body';
import { mergedHeaders, sentHeaders, titleCase, type CodeRequest } from './model';
import { encodedPairs, joined, textSegments, urlSegments, type Segment } from './segments';
import { usedVariables, type CodeValues, type CodeVariable } from './values';

const INDENT = '    ';

/** Windows PowerShell 5.1 refuses these in -Headers; they have parameters of their own. */
const OWN_PARAMETERS = new Map([
  ['content-type', 'ContentType'],
  ['user-agent', 'UserAgent'],
]);

const local = (variable: CodeVariable): string => `$${variable.camel}`;

/** PowerShell also treats curly quotes as quotes, so they are escaped like straight ones. */
function single(text: string): string {
  return `'${text.replace(/['‘’‚‛]/g, '$&$&')}'`;
}

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
    .map((part) => {
      if (part.text !== undefined) {
        return part.text.replace(/[`$"“”„]/g, '`$&');
      }
      return /^\$[A-Za-z0-9_]+$/.test(part.code) ? `\${${part.code.slice(1)}}` : `$(${part.code})`;
    })
    .join('');
  return `"${inner}"`;
}

function text(values: CodeValues, value: string): string {
  return string(textSegments(values, value, local));
}

/** A here-string keeps the body's lines; `'@` or `"@` starting a line would end it early. */
function block(values: CodeValues, value: string): string {
  const segments = joined(textSegments(values, value, local));
  const plain = segments.every((part) => part.text !== undefined);
  const closer = plain ? "'@" : '"@';
  if (!value.includes('\n') || value.split('\n').some((line) => line.startsWith(closer))) {
    return string(segments);
  }
  if (plain) {
    return `@'\n${value}\n'@`;
  }
  const inner = segments
    .map((part) =>
      part.text !== undefined ? part.text.replace(/[`$]/g, '`$&') : `\${${part.code.slice(1)}}`,
    )
    .join('');
  return `@"\n${inner}\n"@`;
}

function hashtable(entries: [string, string][]): string[] {
  const width = Math.max(...entries.map(([key]) => key.length));
  return entries.map(([key, value]) => `${INDENT}${key.padEnd(width)} = ${value}`);
}

export function powershell(request: CodeRequest, values: CodeValues): string {
  const sections: string[][] = [];
  const setup: string[] = [];
  const parameters: [string, string][] = [];
  const encode = (expression: string): string => `[uri]::EscapeDataString(${expression})`;
  setup.push(`$uri = ${string(urlSegments(values, request.url, request.query, local, encode))}`);
  parameters.push(['Uri', '$uri'], ['Method', single(titleCase(request.method))]);

  const headers: [string, string][] = [];
  const signing = request.signing;
  if (request.basic) {
    const pair = textSegments(values, `${request.basic.username}:${request.basic.password}`, local);
    setup.push(
      `$basic = [Convert]::ToBase64String([Text.Encoding]::UTF8.GetBytes(${string(pair)}))`,
    );
    headers.push([single('Authorization'), '"Basic $basic"']);
  } else if (signing?.kind === 'digest') {
    setup.push(
      `$password = ConvertTo-SecureString ${text(values, signing.password)} -AsPlainText -Force`,
      `$credential = New-Object System.Management.Automation.PSCredential (${text(values, signing.username)}, $password)`,
    );
    parameters.push(['Credential', '$credential']);
  } else if (signing?.kind === 'aws') {
    setup.push('# AWS Signature: sign this request with the AWS Tools for PowerShell.');
  }
  for (const header of mergedHeaders(sentHeaders(request))) {
    const own = OWN_PARAMETERS.get(header.name.toLowerCase());
    if (own) {
      parameters.push([own, text(values, header.value)]);
    } else {
      headers.push([text(values, header.name), text(values, header.value)]);
    }
  }
  if (headers.length > 0) {
    setup.push('$headers = @{', ...hashtable(headers), '}');
    parameters.push(['Headers', '$headers']);
  }

  const body = request.body;
  if (body?.mode === 'raw') {
    setup.push(`$body = ${block(values, body.text)}`);
    // 5.1 would send a string body as ISO-8859-1; bytes go out exactly as UTF-8.
    parameters.push(['Body', '[Text.Encoding]::UTF8.GetBytes($body)']);
  } else if (body?.mode === 'urlencoded') {
    setup.push(`$body = ${string(encodedPairs(values, body.fields, local, encode))}`);
    parameters.push(['Body', '$body']);
    if (!parameters.some(([name]) => name === 'ContentType')) {
      parameters.push(['ContentType', single('application/x-www-form-urlencoded')]);
    }
  } else if (body?.mode === 'formData') {
    setup.push(
      '# -Form needs PowerShell 7 or later.',
      '$form = @{',
      ...hashtable(
        body.parts.map((part) => [
          single(part.name),
          part.file ? `Get-Item -LiteralPath ${single(part.value)}` : text(values, part.value),
        ]),
      ),
      '}',
    );
    parameters.push(['Form', '$form']);
  } else if (body?.mode === 'binary') {
    setup.push(`# Sends ${fileName(body.path)} as the body.`);
    parameters.push(['InFile', single(body.path)]);
  }
  sections.push(setup);

  parameters.push(['UseBasicParsing', '$true']);
  const environment = usedVariables(
    values.variables,
    [...setup, ...parameters.map(([, value]) => value)].join('\n'),
    (variable) => variable.camel,
  );
  if (environment.length > 0) {
    sections.unshift([
      '# Values from environment variables',
      ...environment.map((variable) => `$${variable.camel} = $env:${variable.env}`),
    ]);
  }
  sections.push([
    '$request = @{',
    ...hashtable(parameters),
    '}',
    '$response = Invoke-WebRequest @request',
    '$response.StatusCode',
    '$response.Content',
  ]);
  const code = sections.map((section) => section.join('\n')).join('\n\n');
  // Windows PowerShell 5.1 reads a script without a byte order mark as ANSI, mangling é and ✓.
  return Array.from(code).some((character) => (character.codePointAt(0) ?? 0) > 127)
    ? `# Save as UTF-8 with BOM for Windows PowerShell 5.1.\n\n${code}`
    : code;
}
