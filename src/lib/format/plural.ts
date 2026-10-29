/** "1 request", "2 requests". Only for words that take a plain "s". */
export function plural(count: number, word: string): string {
  return `${String(count)} ${word}${count === 1 ? '' : 's'}`;
}
