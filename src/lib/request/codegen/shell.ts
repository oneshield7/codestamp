import { usedVariables, type CodeValues } from './values';

/** Continues a command on the next line, indented. */
export const NEXT = ' \\\n  ';

/** One shell word: single quotes for plain text, double quotes around `${NAME}` references. */
export function word(values: CodeValues, text: string): string {
  const pieces = values.pieces(text);
  if (pieces.every((piece) => piece.variable === undefined)) {
    return `'${text.replaceAll("'", `'\\''`)}'`;
  }
  const inner = pieces
    .map((piece) =>
      piece.variable ? `\${${piece.variable.env}}` : piece.text.replace(/[\\"$`]/g, '\\$&'),
    )
    .join('');
  return `"${inner}"`;
}

/** The command, with a comment naming the environment variables it reads. */
export function withEnvironment(values: CodeValues, command: string): string {
  const environment = usedVariables(values.variables, command, (variable) => variable.env);
  return environment.length === 0
    ? command
    : `# Environment variables: ${environment.map((variable) => variable.env).join(', ')}\n${command}`;
}
