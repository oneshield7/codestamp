import { fileName } from '../body';
import { headerValue, sentHeaders, type CodeRequest } from './model';
import { joined, lines, textSegments, urlSegments, type Segment } from './segments';
import { usedVariables, type CodeValues, type CodeVariable } from './values';

const INDENT = '    ';

/** Headers that belong to the body in .NET, not to the request. */
const CONTENT_HEADERS = new Set([
  'allow',
  'content-disposition',
  'content-encoding',
  'content-language',
  'content-length',
  'content-location',
  'content-md5',
  'content-range',
  'content-type',
  'expires',
  'last-modified',
]);

const local = (variable: CodeVariable): string => variable.camel;

/** A JSON string literal is also a C# one; an interpolated string doubles its braces. */
function string(segments: Segment[]): string {
  const parts = joined(segments);
  const [only] = parts;
  if (parts.length === 1 && only?.code !== undefined) {
    return only.code;
  }
  if (parts.every((part) => part.text !== undefined)) {
    return JSON.stringify(parts.map((part) => part.text).join(''));
  }
  const inner = parts
    .map((part) =>
      part.text !== undefined
        ? JSON.stringify(part.text).slice(1, -1).replace(/[{}]/g, '$&$&')
        : `{${part.code}}`,
    )
    .join('');
  return `$"${inner}"`;
}

function text(values: CodeValues, value: string): string {
  return string(textSegments(values, value, local));
}

function longestRun(text: string, character: string): number {
  let longest = 0;
  let run = 0;
  for (const current of text) {
    run = current === character ? run + 1 : 0;
    longest = Math.max(longest, run);
  }
  return longest;
}

/**
 * A raw string literal keeps the body's lines (C# 11, .NET 7 or later). Its quotes outnumber any
 * run of quotes inside, and with variables its `$` signs outnumber any run of braces.
 */
function block(values: CodeValues, value: string): string {
  const parts = joined(textSegments(values, value, local));
  const plainText = parts.map((part) => part.text ?? '').join('');
  const quotes = '"'.repeat(Math.max(3, longestRun(plainText, '"') + 1));
  const plain = parts.every((part) => part.text !== undefined);
  const dollars = plain ? 0 : Math.max(longestRun(plainText, '{'), longestRun(plainText, '}')) + 1;
  const open = '{'.repeat(dollars);
  const close = '}'.repeat(dollars);
  const inner = parts
    .map((part) => (part.text !== undefined ? part.text : `${open}${part.code}${close}`))
    .join('');
  const body = lines(inner)
    .map((line) => (line === '' ? '' : `${INDENT}${line}`))
    .join('\n');
  return `${'$'.repeat(dollars)}${quotes}\n${body}\n${INDENT}${quotes}`;
}

export function csharp(request: CodeRequest, values: CodeValues): string {
  const usings = new Set<string>();
  const encode = (expression: string): string => `Uri.EscapeDataString(${expression})`;
  const signing = request.signing;

  const client: string[] = [];
  if (signing?.kind === 'digest') {
    usings.add('System.Net');
    client.push(
      'using var handler = new HttpClientHandler',
      '{',
      `${INDENT}Credentials = new NetworkCredential(${text(values, signing.username)}, ${text(values, signing.password)}),`,
      '};',
      'using var client = new HttpClient(handler);',
    );
  } else {
    client.push('using var client = new HttpClient();');
  }

  const setup = [
    `var url = ${string(urlSegments(values, request.url, request.query, local, encode))};`,
    `using var request = new HttpRequestMessage(new HttpMethod(${JSON.stringify(request.method)}), url);`,
  ];
  if (request.basic) {
    usings.add('System.Net.Http.Headers').add('System.Text');
    const pair = text(values, `${request.basic.username}:${request.basic.password}`);
    setup.push(
      `var basic = Convert.ToBase64String(Encoding.UTF8.GetBytes(${pair}));`,
      'request.Headers.Authorization = new AuthenticationHeaderValue("Basic", basic);',
    );
  } else if (signing?.kind === 'aws') {
    setup.push('// AWS Signature: sign this request with the AWS SDK for .NET (AWS4Signer).');
  }
  const headers = sentHeaders(request);
  for (const header of headers) {
    if (!CONTENT_HEADERS.has(header.name.toLowerCase())) {
      setup.push(
        `request.Headers.TryAddWithoutValidation(${text(values, header.name)}, ${text(values, header.value)});`,
      );
    }
  }

  const body = request.body;
  if (body?.mode === 'raw') {
    usings.add('System.Text');
    const content = body.text.includes('\n') ? block(values, body.text) : text(values, body.text);
    setup.push(`request.Content = new StringContent(${content}, Encoding.UTF8);`);
  } else if (body?.mode === 'urlencoded') {
    setup.push(
      'request.Content = new FormUrlEncodedContent(new[]',
      '{',
      ...body.fields.map(
        (field) =>
          `${INDENT}KeyValuePair.Create(${text(values, field.name)}, ${text(values, field.value)}),`,
      ),
      '});',
    );
  } else if (body?.mode === 'formData') {
    setup.push('var form = new MultipartFormDataContent();');
    for (const part of body.parts) {
      setup.push(
        part.file
          ? `form.Add(new StreamContent(File.OpenRead(${JSON.stringify(part.value)})), ${text(values, part.name)}, ${JSON.stringify(fileName(part.value))});`
          : `form.Add(new StringContent(${text(values, part.value)}), ${text(values, part.name)});`,
      );
    }
    setup.push('request.Content = form;');
  } else if (body?.mode === 'binary') {
    setup.push(`request.Content = new StreamContent(File.OpenRead(${JSON.stringify(body.path)}));`);
  }
  if (body && body.mode !== 'formData') {
    const type = headerValue(headers, 'content-type');
    if (type !== undefined) {
      usings.add('System.Net.Http.Headers');
      setup.push(
        `request.Content.Headers.ContentType = MediaTypeHeaderValue.Parse(${text(values, type)});`,
      );
    }
    for (const header of headers) {
      const name = header.name.toLowerCase();
      if (CONTENT_HEADERS.has(name) && name !== 'content-type') {
        setup.push(
          `request.Content.Headers.TryAddWithoutValidation(${text(values, header.name)}, ${text(values, header.value)});`,
        );
      }
    }
  }

  const send = [
    'using var response = await client.SendAsync(request);',
    'Console.WriteLine((int)response.StatusCode);',
    'Console.WriteLine(await response.Content.ReadAsStringAsync());',
  ];
  const sections = [client, setup, send];
  const environment = usedVariables(values.variables, [...client, ...setup].join('\n'), local);
  if (environment.length > 0) {
    sections.unshift(
      environment.map(
        (variable) =>
          `var ${variable.camel} = Environment.GetEnvironmentVariable(${JSON.stringify(variable.env)});`,
      ),
    );
  }
  if (usings.size > 0) {
    sections.unshift([...usings].sort().map((name) => `using ${name};`));
  }
  return sections.map((section) => section.join('\n')).join('\n\n');
}
