import { listExamples, type ExampleSummary } from '../api/examples';

/** The saved examples of the open request, shared by its response panel and Examples tab. */
export class ExampleList {
  items = $state<ExampleSummary[]>([]);
  #requestId: string | null = null;

  async load(requestId: string): Promise<void> {
    this.#requestId = requestId;
    this.items = await listExamples(requestId);
  }

  async refresh(): Promise<void> {
    if (this.#requestId !== null) {
      this.items = await listExamples(this.#requestId);
    }
  }
}
