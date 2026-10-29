import { sentHeaders, type CodeRequest } from './model';
import { NEXT, withEnvironment, word } from './shell';
import type { CodeValues } from './values';

/** curl reads `@"…"` file names with backslash escapes, which also keeps Windows paths intact. */
function fileWord(path: string): string {
  return `"${path.replaceAll('\\', '\\\\').replaceAll('"', '\\"')}"`;
}

/** curl encodes the value after `=` itself; the name has to arrive encoded already. */
function encodedName(values: CodeValues, name: string): string {
  return values
    .pieces(name)
    .map((piece) =>
      piece.variable ? `{{${piece.variable.name}}}` : encodeURIComponent(piece.text),
    )
    .join('');
}

export function curl(request: CodeRequest, values: CodeValues): string {
  const url = word(values, request.url);
  const parts = [
    request.method === 'GET'
      ? `curl ${url}`
      : request.method === 'HEAD'
        ? `curl --head ${url}`
        : `curl --request ${request.method} ${url}`,
  ];
  for (const pair of request.query) {
    parts.push(`--url-query ${word(values, `${encodedName(values, pair.name)}=${pair.value}`)}`);
  }
  const signing = request.signing;
  if (request.basic) {
    parts.push(`--user ${word(values, `${request.basic.username}:${request.basic.password}`)}`);
  } else if (signing?.kind === 'digest') {
    parts.push(`--digest --user ${word(values, `${signing.username}:${signing.password}`)}`);
  } else if (signing?.kind === 'aws') {
    parts.push(
      `--aws-sigv4 ${word(values, `aws:amz:${signing.region}:${signing.service}`)}`,
      `--user ${word(values, `${signing.accessKey}:${signing.secretKey}`)}`,
    );
    if (signing.sessionToken !== '') {
      parts.push(`--header ${word(values, `x-amz-security-token: ${signing.sessionToken}`)}`);
    }
  }
  for (const header of sentHeaders(request)) {
    parts.push(`--header ${word(values, `${header.name}: ${header.value}`)}`);
  }
  const body = request.body;
  if (body?.mode === 'raw') {
    parts.push(`--data-raw ${word(values, body.text)}`);
  } else if (body?.mode === 'urlencoded') {
    for (const field of body.fields) {
      parts.push(
        `--data-urlencode ${word(values, `${encodedName(values, field.name)}=${field.value}`)}`,
      );
    }
  } else if (body?.mode === 'formData') {
    for (const part of body.parts) {
      parts.push(
        part.file
          ? `--form ${word(values, `${part.name}=@${fileWord(part.value)}`)}`
          : `--form-string ${word(values, `${part.name}=${part.value}`)}`,
      );
    }
  } else if (body?.mode === 'binary') {
    parts.push(`--data-binary ${word(values, `@${body.path}`)}`);
  }
  return withEnvironment(values, parts.join(NEXT));
}
