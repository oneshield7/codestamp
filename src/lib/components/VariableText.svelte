<script lang="ts">
  import { splitVariables } from '../format/variables';

  interface Props {
    text: string;
    /** Names that are defined; `null` marks none as missing. */
    known: ReadonlySet<string> | null;
  }

  let { text, known }: Props = $props();

  const parts = $derived(splitVariables(text));
</script>

{#each parts as part, index (index)}{#if part.variable === null}{part.text}{:else}<span
      class="var"
      class:missing={known !== null && !known.has(part.variable)}
      title={known !== null && !known.has(part.variable)
        ? `{{${part.variable}}} is not defined`
        : undefined}>{part.text}</span
    >{/if}{/each}

<style>
  .var {
    border-radius: 3px;
    background: var(--var-bg);
    color: var(--var);
  }

  .var.missing {
    background: var(--err-bg);
    color: var(--err);
    text-decoration: underline wavy var(--err);
    text-decoration-thickness: 1px;
    text-underline-offset: 3px;
  }
</style>
