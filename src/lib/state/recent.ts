/** Values kept for this session; once there are more than `limit`, the least recent leaves. */
export class RecentCache<T> {
  readonly #items = new Map<string, T>();
  readonly #limit: number;

  constructor(limit: number) {
    this.#limit = limit;
  }

  get(key: string): T | undefined {
    return this.#items.get(key);
  }

  set(key: string, value: T): void {
    this.#items.delete(key);
    this.#items.set(key, value);
    const oldest = this.#items.keys().next().value;
    if (this.#items.size > this.#limit && oldest !== undefined) {
      this.#items.delete(oldest);
    }
  }
}
