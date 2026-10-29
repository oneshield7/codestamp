import { joined, textSegments, type Segment } from './segments';
import type { CodeValues, CodeVariable } from './values';

export const jsLocal = (variable: CodeVariable): string => variable.camel;

/** A JSON string literal, or a template literal around `${…}` and line breaks. */
export function jsString(segments: Segment[]): string {
  const parts = joined(segments);
  const [only] = parts;
  if (parts.length === 1 && only?.code !== undefined) {
    return only.code;
  }
  const plain = parts.every((part) => part.text !== undefined);
  const whole = parts.map((part) => part.text ?? '').join('');
  if (plain && !whole.includes('\n')) {
    return JSON.stringify(whole);
  }
  const inner = parts
    .map((part) =>
      part.text !== undefined
        ? part.text.replace(/\\/g, '\\\\').replace(/`/g, '\\`').replace(/\$\{/g, '\\${')
        : `\${${part.code}}`,
    )
    .join('');
  return `\`${inner}\``;
}

export function jsText(values: CodeValues, value: string): string {
  return jsString(textSegments(values, value, jsLocal));
}
