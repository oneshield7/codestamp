/** Bumped whenever saved variables or the active environment change, so previews refresh. */
class VariableRevision {
  value = $state(0);

  bump(): void {
    this.value += 1;
  }
}

export const variableRevision = new VariableRevision();
