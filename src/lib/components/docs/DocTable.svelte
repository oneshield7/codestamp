<script lang="ts">
  interface Props {
    title: string;
    rows: { name: string; value: string }[];
  }

  let { title, rows }: Props = $props();
</script>

{#if rows.length > 0}
  <section class="block" aria-label={title}>
    <h5>{title}</h5>
    <div class="wrap">
      <table>
        <thead>
          <tr><th scope="col">Name</th><th scope="col">Value</th></tr>
        </thead>
        <tbody>
          {#each rows as row, index (index)}
            <tr>
              <td class="mono">{row.name}</td>
              <td class="mono">{row.value}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    </div>
  </section>
{/if}

<style>
  .block {
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  h5 {
    margin: 0;
    font-size: 12px;
    font-weight: 600;
    color: var(--text-muted);
  }

  .wrap {
    overflow-x: auto;
  }

  table {
    width: 100%;
    border-collapse: collapse;
  }

  th,
  td {
    padding: 6px 10px;
    border-bottom: 1px solid var(--border-soft);
    text-align: left;
    vertical-align: top;
  }

  th {
    color: var(--text-muted);
    font-size: 12px;
    font-weight: 500;
  }

  th:first-child,
  td:first-child {
    width: 34%;
  }

  td {
    color: var(--text-2);
    font-size: 12px;
    overflow-wrap: anywhere;
  }
</style>
