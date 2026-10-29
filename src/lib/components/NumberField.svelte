<script lang="ts">
  import { isInRange, type NumberRange } from '../appearance/presets';

  interface Props {
    label: string;
    value: number;
    range: NumberRange;
    /** Digits after the decimal point that are allowed (0 = whole numbers). */
    decimals?: number;
    onChange: (value: number) => void;
  }

  let { label, value, range, decimals = 0, onChange }: Props = $props();

  const errorId = `number-error-${crypto.randomUUID()}`;
  const step = $derived(decimals === 0 ? 1 : 1 / 10 ** decimals);
  let invalid = $state(false);

  // Out-of-range or overly precise values are refused, not corrected, so the field shows why.
  function handleInput(event: Event & { currentTarget: HTMLInputElement }): void {
    const next = event.currentTarget.valueAsNumber;
    invalid = !isInRange(next, range) || Number(next.toFixed(decimals)) !== next;
    if (!invalid) {
      onChange(next);
    }
  }
</script>

<label class="field">
  {label}
  <input
    class="input"
    type="number"
    min={range.min}
    max={range.max}
    {step}
    {value}
    aria-invalid={invalid}
    aria-describedby={invalid ? errorId : undefined}
    oninput={handleInput}
  />
  {#if invalid}
    <span class="error" id={errorId}>
      Use {range.min} to {range.max}{decimals === 0
        ? ', whole numbers'
        : ` in steps of ${String(step)}`}
    </span>
  {/if}
</label>

<style>
  .field {
    flex: 1 1 110px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 13px;
    color: var(--text-2);
  }

  .input {
    height: 40px;
    padding: 0 12px;
    border: 1px solid var(--border-strong);
    border-radius: var(--radius-md);
    background: var(--surface);
    font-size: 14px;
  }

  .input[aria-invalid='true'] {
    border-color: var(--err);
  }

  .error {
    font-size: 12px;
    color: var(--err);
  }
</style>
