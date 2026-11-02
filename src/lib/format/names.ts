/** The longest name of a collection, section or request; the Rust core checks it again. */
export const MAX_NAME_LENGTH = 120;

const COPY_SUFFIX = ' copy';

/** "Name copy", shortened so it still fits the limit (counted in characters, like the core). */
export function copyName(name: string): string {
  const room = MAX_NAME_LENGTH - COPY_SUFFIX.length;
  return `${Array.from(name.trim()).slice(0, room).join('').trimEnd()}${COPY_SUFFIX}`;
}

/** Whether `name` is already used by one of `names`, ignoring case and outer spaces. */
export function isNameTaken(name: string, names: readonly string[]): boolean {
  const wanted = name.trim().toLocaleLowerCase();
  return names.some((existing) => existing.trim().toLocaleLowerCase() === wanted);
}

/** `base`, or `base 2`, `base 3`, … whichever is not taken yet. */
export function freeName(base: string, names: readonly string[]): string {
  let name = base;
  for (let number = 2; isNameTaken(name, names); number += 1) {
    name = `${base} ${String(number)}`;
  }
  return name;
}

/** The first character of a name, emoji included, uppercase: what stands in for its icon. */
export function initial(name: string): string {
  return (Array.from(name.trim())[0] ?? '?').toUpperCase();
}
