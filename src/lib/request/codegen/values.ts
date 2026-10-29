import { splitVariables } from '../../format/variables';
import type { CodeRequest } from './model';

/** Where the generated code gets `{{name}}` values from. */
export type ValueSource = 'environment' | 'template' | 'filled';

/** A `{{name}}` that the generated code reads from an environment variable. */
export interface CodeVariable {
  name: string;
  /** The environment variable, such as `BASE_URL`. */
  env: string;
  /** The local name in camelCase (`baseUrl`) and snake_case (`base_url`). */
  camel: string;
  snake: string;
}

/** Text written as it is, or a variable read when the code runs. */
export type Piece = { text: string; variable?: never } | { variable: CodeVariable; text?: never };

/**
 * Keywords of the languages Codestamp writes, plus the names the generated code uses for its own
 * values, so a `{{name}}` never shadows either.
 */
const TAKEN = new Set(
  (
    'abstract and as assert async await base bool boolean break byte case catch chan char ' +
    'checked class const continue decimal def default defer del delegate do double elif else ' +
    'enum event except explicit export extends extern false final finally fixed float for ' +
    'foreach from func function global go goto if implicit import in instanceof int interface ' +
    'internal is lambda let lock long map namespace native new nonlocal not null object operator ' +
    'or out override package params pass private protected public raise range readonly ref ' +
    'return sbyte sealed select short sizeof stackalloc static strictfp string struct super ' +
    'switch synchronized this throw throws transient true try type typeof uint ulong unchecked ' +
    'unsafe ushort using var virtual void volatile while with yield ' +
    'alias begin box crate defined dyn elsif end ensure fn impl loop match mod module move mut nil ' +
    'pub redo rescue retry self then trait undef unless until when where ' +
    'basic body boundary client config content credential data encoded env environment err error file ' +
    'files form headers http json params part password payload query reader req request resp response ' +
    'unreserved uri url writer'
  ).split(' '),
);

/**
 * Variables the system already sets. `{{user}}` read from USER would quietly get the login name,
 * so these names get a `_VALUE` suffix.
 */
const SYSTEM_ENVIRONMENT = new Set(
  (
    'APPDATA COMPUTERNAME COMSPEC DISPLAY EDITOR HOME HOMEDRIVE HOMEPATH HOSTNAME LANG ' +
    'LOCALAPPDATA LOGNAME MAIL OLDPWD OS PATH PATHEXT PROGRAMFILES PWD SHELL SYSTEMROOT TEMP ' +
    'TERM TMP TMPDIR USER USERNAME USERPROFILE WINDIR'
  ).split(' '),
);

export class CodeValues {
  readonly variables: CodeVariable[] = [];
  readonly #byName = new Map<string, CodeVariable>();

  constructor(
    readonly fromEnvironment: boolean,
    request: CodeRequest,
  ) {
    if (!fromEnvironment) {
      return;
    }
    for (const text of requestTexts(request)) {
      for (const part of splitVariables(text)) {
        if (part.variable !== null && !part.variable.startsWith('$')) {
          this.#add(part.variable);
        }
      }
    }
  }

  /** The text split into plain parts and variables; one plain part unless reading the environment. */
  pieces(text: string): Piece[] {
    if (!this.fromEnvironment) {
      return text === '' ? [] : [{ text }];
    }
    const pieces: Piece[] = [];
    for (const part of splitVariables(text)) {
      const variable = part.variable === null ? undefined : this.#byName.get(part.variable);
      const last = pieces.at(-1);
      if (variable) {
        pieces.push({ variable });
      } else if (last?.text !== undefined) {
        last.text += part.text;
      } else {
        pieces.push({ text: part.text });
      }
    }
    return pieces;
  }

  hasVariables(text: string): boolean {
    return this.pieces(text).some((piece) => piece.variable !== undefined);
  }

  #add(name: string): void {
    if (this.#byName.has(name)) {
      return;
    }
    const parts = words(name);
    const startsWithDigit = /^[0-9]/.test(parts[0] ?? '');
    const lead = startsWithDigit ? ['value', ...parts] : parts.length === 0 ? ['value'] : parts;
    const camelBase = lead
      .map((word, index) =>
        index === 0
          ? word.toLowerCase()
          : word.charAt(0).toUpperCase() + word.slice(1).toLowerCase(),
      )
      .join('');
    const snakeBase = lead.map((word) => word.toLowerCase()).join('_');
    const envBase = lead.map((word) => word.toUpperCase()).join('_');
    const variable = {
      name,
      env: this.#unique(
        SYSTEM_ENVIRONMENT.has(envBase) ? `${envBase}_VALUE` : envBase,
        (item) => item.env,
      ),
      camel: this.#unique(
        TAKEN.has(camelBase) ? `${camelBase}Value` : camelBase,
        (item) => item.camel,
      ),
      snake: this.#unique(
        TAKEN.has(snakeBase) ? `${snakeBase}_value` : snakeBase,
        (item) => item.snake,
      ),
    };
    this.variables.push(variable);
    this.#byName.set(name, variable);
  }

  /** `base-url` and `baseUrl` would both become `BASE_URL`; the second gets a number. */
  #unique(base: string, read: (variable: CodeVariable) => string): string {
    const used = new Set(this.variables.map(read));
    let candidate = base;
    for (let count = 2; used.has(candidate); count += 1) {
      candidate = `${base}${String(count)}`;
    }
    return candidate;
  }
}

/**
 * The variables the code refers to by the name it gives them, such as `$token`. Auth that only
 * gets a comment would otherwise leave unused values behind, which Go refuses to compile.
 */
export function usedVariables(
  variables: CodeVariable[],
  code: string,
  nameOf: (variable: CodeVariable) => string,
): CodeVariable[] {
  return variables.filter((variable) => mentions(code, nameOf(variable)));
}

/** Whether `name` appears in `code` as a whole word, not as part of a longer name. */
function mentions(code: string, name: string): boolean {
  const isWordCharacter = (character: string): boolean => /^[A-Za-z0-9_]$/.test(character);
  for (let at = code.indexOf(name); at >= 0; at = code.indexOf(name, at + 1)) {
    if (!isWordCharacter(code.charAt(at - 1)) && !isWordCharacter(code.charAt(at + name.length))) {
      return true;
    }
  }
  return false;
}

/** `baseURL` and `base_url` both give `base`, `url`. */
function words(name: string): string[] {
  return name
    .replace(/([a-z0-9])([A-Z])/g, '$1 $2')
    .split(/[^A-Za-z0-9]+/)
    .filter((word) => word !== '');
}

/** Every text the code sends, in the order the code shows it, so variables are listed that way. */
function requestTexts(request: CodeRequest): string[] {
  const texts = [request.url];
  for (const pair of request.query) {
    texts.push(pair.name, pair.value);
  }
  if (request.basic) {
    texts.push(request.basic.username, request.basic.password);
  }
  const signing = request.signing;
  if (signing?.kind === 'digest') {
    texts.push(signing.username, signing.password);
  } else if (signing?.kind === 'aws') {
    texts.push(signing.accessKey, signing.secretKey, signing.sessionToken);
  }
  for (const header of request.headers) {
    texts.push(header.name, header.value);
  }
  const body = request.body;
  if (body?.mode === 'raw') {
    texts.push(body.text);
  } else if (body?.mode === 'urlencoded') {
    for (const field of body.fields) {
      texts.push(field.name, field.value);
    }
  } else if (body?.mode === 'formData') {
    for (const part of body.parts) {
      texts.push(part.name, part.file ? '' : part.value);
    }
  }
  return texts;
}
