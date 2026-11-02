/** Bumped whenever saved variables or the active environment change, so previews refresh. */
class VariableRevision {
  value = $state(0);

  #sharedChanged: (collectionId: string) => void = () => undefined;

  bump(): void {
    this.value += 1;
  }

  /** Told when variables a collection shares change, so its next sync takes them along. */
  watchShared(listener: (collectionId: string) => void): void {
    this.#sharedChanged = listener;
  }

  sharedChanged(collectionId: string): void {
    this.#sharedChanged(collectionId);
  }
}

export const variableRevision = new VariableRevision();
