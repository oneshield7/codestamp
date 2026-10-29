import { sentHeaders, type CodeRequest } from './model';
import { NEXT, withEnvironment, word } from './shell';
import type { CodeValues } from './values';

/** A separator inside a name is escaped, so HTTPie does not split the item there. */
function escapedName(name: string): string {
  return name.replace(/[:=@\\]/g, '\\$&');
}

export function httpie(request: CodeRequest, values: CodeValues): string {
  const body = request.body;
  const flags = [
    // Otherwise HTTPie takes a script's stdin as the body and may wait on it forever.
    ...(body?.mode === 'binary' ? [] : ['--ignore-stdin']),
    ...(body?.mode === 'urlencoded' ? ['--form'] : []),
    ...(body?.mode === 'formData' ? ['--multipart'] : []),
    '--print=hb',
  ];
  const parts = [`http ${flags.join(' ')} ${request.method} ${word(values, request.url)}`];
  for (const pair of request.query) {
    parts.push(word(values, `${escapedName(pair.name)}==${pair.value}`));
  }
  const signing = request.signing;
  if (request.basic) {
    parts.push(`--auth ${word(values, `${request.basic.username}:${request.basic.password}`)}`);
  } else if (signing?.kind === 'digest') {
    parts.push(
      `--auth-type digest --auth ${word(values, `${signing.username}:${signing.password}`)}`,
    );
  }
  for (const header of sentHeaders(request)) {
    // `Name:` with nothing after it would remove the header; `Name;` sends it empty.
    parts.push(
      word(
        values,
        header.value === ''
          ? `${escapedName(header.name)};`
          : `${escapedName(header.name)}:${header.value}`,
      ),
    );
  }
  if (body?.mode === 'raw') {
    parts.push(`--raw ${word(values, body.text)}`);
  } else if (body?.mode === 'urlencoded') {
    for (const field of body.fields) {
      parts.push(word(values, `${escapedName(field.name)}=${field.value}`));
    }
  } else if (body?.mode === 'formData') {
    for (const part of body.parts) {
      parts.push(
        word(
          values,
          part.file
            ? `${escapedName(part.name)}@${part.value}`
            : `${escapedName(part.name)}=${part.value}`,
        ),
      );
    }
  } else if (body?.mode === 'binary') {
    parts.push(`< ${word(values, body.path)}`);
  }
  const command = withEnvironment(values, parts.join(NEXT));
  return signing?.kind === 'aws'
    ? `# AWS Signature: sign with the httpie-aws-authv4 plugin (--auth-type aws4).\n${command}`
    : command;
}
