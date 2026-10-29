import { escaped, hex } from './escape';
import { sentHeaders, type CodeRequest } from './model';
import { joined, textSegments, type Segment } from './segments';
import { usedVariables, type CodeValues, type CodeVariable } from './values';

const I1 = '    ';
const I2 = I1.repeat(2);
const I3 = I1.repeat(3);

const local = (variable: CodeVariable): string => variable.snake;

const SPECIAL = new Map([['"', '\\"']]);

/**
 * How a value is passed: `any` where reqwest takes a reference or a String, `str` inside arrays
 * whose items must share the type `&str`, `owned` where it needs a String of its own.
 */
type Use = 'any' | 'str' | 'owned';

function control(code: number): string {
  return `\\u{${hex(code, 1)}}`;
}

/** The longest run of `#` right after a quote, which a raw string's own `#`s must outnumber. */
function hashesFor(text: string): string {
  let longest = 0;
  for (const match of text.matchAll(/"(#+)/g)) {
    longest = Math.max(longest, match[1]?.length ?? 0);
  }
  return '#'.repeat(longest + 1);
}

/** Raw strings keep lines as they are; Rust refuses a lone carriage return in one. */
function canBeRaw(text: string): boolean {
  return text.includes('\n') && !text.includes('\r');
}

/** reqwest takes a `&'static str` literal wherever it wants an owned string as well. */
function literal(text: string): string {
  const hashes = hashesFor(text);
  return canBeRaw(text) ? `r${hashes}"${text}"${hashes}` : `"${escaped(text, control, SPECIAL)}"`;
}

/** A plain literal, or `format!` with the variables named inside it (Rust 2021). */
function string(segments: Segment[], use: Use): string {
  const parts = joined(segments);
  const [only] = parts;
  if (parts.length === 1 && only?.code !== undefined) {
    return use === 'owned'
      ? `${only.code}.clone()`
      : use === 'str'
        ? `${only.code}.as_str()`
        : `&${only.code}`;
  }
  const whole = parts.map((part) => part.text ?? '').join('');
  if (parts.every((part) => part.text !== undefined)) {
    return literal(whole);
  }
  const raw = canBeRaw(whole);
  const hashes = hashesFor(whole);
  const inner = parts
    .map((part) => {
      if (part.text === undefined) {
        return `{${part.code}}`;
      }
      const braced = part.text.replace(/[{}]/g, '$&$&');
      return raw ? braced : escaped(braced, control, SPECIAL);
    })
    .join('');
  const formatted = raw ? `format!(r${hashes}"${inner}"${hashes})` : `format!("${inner}")`;
  return use === 'str' ? `${formatted}.as_str()` : formatted;
}

function text(values: CodeValues, value: string, use: Use = 'any'): string {
  return string(textSegments(values, value, local), use);
}

function pairList(values: CodeValues, items: { name: string; value: string }[]): string[] {
  return [
    ...items.map(
      (item) => `${I3}(${text(values, item.name, 'str')}, ${text(values, item.value, 'str')}),`,
    ),
  ];
}

export function rust(request: CodeRequest, values: CodeValues): string {
  const setup: string[] = [`${I1}let client = reqwest::blocking::Client::new();`];
  const builder = [
    `${I1}let request = client`,
    `${I2}.request(reqwest::Method::${request.method}, ${text(values, request.url)})`,
  ];
  if (request.query.length > 0) {
    builder.push(`${I2}.query(&[`, ...pairList(values, request.query), `${I2}])`);
  }
  const signing = request.signing;
  if (request.basic) {
    builder.push(
      `${I2}.basic_auth(${text(values, request.basic.username)}, Some(${text(values, request.basic.password)}))`,
    );
  } else if (signing?.kind === 'digest') {
    setup.push(
      `${I1}// Digest auth: reqwest cannot answer the server's challenge; use the diqwest crate.`,
    );
  } else if (signing?.kind === 'aws') {
    setup.push(`${I1}// AWS Signature: sign this request with the aws-sigv4 crate.`);
  }
  for (const header of sentHeaders(request)) {
    builder.push(
      `${I2}.header(${text(values, header.name, 'str')}, ${text(values, header.value, 'str')})`,
    );
  }

  const body = request.body;
  if (body?.mode === 'raw') {
    setup.push(`${I1}let body = ${text(values, body.text, 'owned')};`);
    builder.push(`${I2}.body(body)`);
  } else if (body?.mode === 'urlencoded') {
    builder.push(`${I2}.form(&[`, ...pairList(values, body.fields), `${I2}])`);
  } else if (body?.mode === 'formData') {
    const parts = body.parts.map((part) =>
      part.file
        ? `${I2}.file(${text(values, part.name, 'owned')}, ${literal(part.value)})?`
        : `${I2}.text(${text(values, part.name, 'owned')}, ${text(values, part.value, 'owned')})`,
    );
    setup.push(`${I1}let form = reqwest::blocking::multipart::Form::new()`, ...parts);
    setup.push(`${setup.pop() ?? ''};`);
    builder.push(`${I2}.multipart(form)`);
  } else if (body?.mode === 'binary') {
    setup.push(`${I1}let body = std::fs::read(${literal(body.path)})?;`);
    builder.push(`${I2}.body(body)`);
  }
  builder.push(`${I2}.build()?;`);

  const send = [
    `${I1}let response = client.execute(request)?;`,
    `${I1}println!("{}", response.status().as_u16());`,
    `${I1}println!("{}", response.text()?);`,
    `${I1}Ok(())`,
  ];
  const sections = [setup, builder, send];
  const environment = usedVariables(values.variables, [...setup, ...builder].join('\n'), local);
  if (environment.length > 0) {
    sections.unshift(
      environment.map(
        (variable) =>
          `${I1}let ${variable.snake} = std::env::var(${JSON.stringify(variable.env)})?;`,
      ),
    );
  }
  // reqwest 0.13 keeps `query`, `form` and `multipart` behind features of their own.
  const features = [
    'blocking',
    ...(request.query.length > 0 ? ['query'] : []),
    ...(body?.mode === 'urlencoded' ? ['form'] : []),
    ...(body?.mode === 'formData' ? ['multipart'] : []),
  ]
    .map((feature) => JSON.stringify(feature))
    .join(', ');
  return [
    `// Cargo.toml: reqwest = { version = "0.13", features = [${features}] }`,
    'fn main() -> Result<(), Box<dyn std::error::Error>> {',
    sections.map((section) => section.join('\n')).join('\n\n'),
    '}',
  ].join('\n');
}
