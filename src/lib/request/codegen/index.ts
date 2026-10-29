import { axios } from './axios';
import { csharp } from './csharp';
import { curl } from './curl';
import { dart } from './dart';
import { fetchCode } from './fetch';
import { goCode } from './go';
import { httpie } from './httpie';
import { java } from './java';
import { kotlin } from './kotlin';
import type { CodeRequest } from './model';
import { php } from './php';
import { powershell } from './powershell';
import { python } from './python';
import { ruby } from './ruby';
import { rust } from './rust';
import { swift } from './swift';
import { CodeValues, type ValueSource } from './values';

export { filledRequest, templateRequest, type CodeRequest } from './model';
export type { ValueSource } from './values';

export type CodeLanguage =
  | 'curl'
  | 'httpie'
  | 'powershell'
  | 'fetch'
  | 'axios'
  | 'python'
  | 'php'
  | 'go'
  | 'java'
  | 'kotlin'
  | 'csharp'
  | 'dart'
  | 'swift'
  | 'ruby'
  | 'rust';

/** Shell tools first, then JavaScript, then the other languages. */
export const CODE_LANGUAGES: {
  language: CodeLanguage;
  label: string;
  /** The highlighting the code viewer has for it. */
  editor: 'javascript' | 'text';
}[] = [
  { language: 'curl', label: 'cURL', editor: 'text' },
  { language: 'httpie', label: 'HTTPie', editor: 'text' },
  { language: 'powershell', label: 'PowerShell', editor: 'text' },
  { language: 'fetch', label: 'JavaScript fetch', editor: 'javascript' },
  { language: 'axios', label: 'JavaScript axios', editor: 'javascript' },
  { language: 'python', label: 'Python requests', editor: 'text' },
  { language: 'php', label: 'PHP Guzzle', editor: 'text' },
  { language: 'go', label: 'Go net/http', editor: 'text' },
  { language: 'java', label: 'Java HttpClient', editor: 'text' },
  { language: 'kotlin', label: 'Kotlin OkHttp', editor: 'text' },
  { language: 'csharp', label: 'C# HttpClient', editor: 'text' },
  { language: 'dart', label: 'Dart http', editor: 'text' },
  { language: 'swift', label: 'Swift URLSession', editor: 'text' },
  { language: 'ruby', label: 'Ruby Net::HTTP', editor: 'text' },
  { language: 'rust', label: 'Rust reqwest', editor: 'text' },
];

function generator(language: CodeLanguage): (request: CodeRequest, values: CodeValues) => string {
  switch (language) {
    case 'curl':
      return curl;
    case 'httpie':
      return httpie;
    case 'powershell':
      return powershell;
    case 'fetch':
      return fetchCode;
    case 'axios':
      return axios;
    case 'python':
      return python;
    case 'php':
      return php;
    case 'go':
      return goCode;
    case 'java':
      return java;
    case 'kotlin':
      return kotlin;
    case 'csharp':
      return csharp;
    case 'dart':
      return dart;
    case 'swift':
      return swift;
    case 'ruby':
      return ruby;
    case 'rust':
      return rust;
  }
}

export function isCodeLanguage(value: unknown): value is CodeLanguage {
  return CODE_LANGUAGES.some((option) => option.language === value);
}

/** Each `{{name}}` and the environment variable the code reads it from, in the order used. */
export function environmentVariables(
  request: CodeRequest,
  source: ValueSource,
): { name: string; env: string }[] {
  return new CodeValues(source === 'environment', request).variables.map(({ name, env }) => ({
    name,
    env,
  }));
}

export function generateCode(
  language: CodeLanguage,
  request: CodeRequest,
  source: ValueSource,
): string {
  return generator(language)(request, new CodeValues(source === 'environment', request));
}
