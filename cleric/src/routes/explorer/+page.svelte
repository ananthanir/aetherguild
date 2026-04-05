<script lang="ts">
  import { Search, ChevronRight, Box } from "lucide-svelte";

  let searchQuery = $state("");

  const mockBlocks = [
    { number: 5, timestamp: "2 min ago", txCount: 3, gasUsed: "126,000" },
    { number: 4, timestamp: "5 min ago", txCount: 1, gasUsed: "21,000" },
    { number: 3, timestamp: "8 min ago", txCount: 2, gasUsed: "84,000" },
    { number: 2, timestamp: "12 min ago", txCount: 1, gasUsed: "53,000" },
    { number: 1, timestamp: "15 min ago", txCount: 0, gasUsed: "0" },
    { number: 0, timestamp: "20 min ago", txCount: 0, gasUsed: "0" },
  ];
</script>

<div class="flex-1 overflow-y-auto">
<div class="p-6">
  <div class="mb-6">
    <h1 class="text-xl font-semibold text-text">Explorer</h1>
    <p class="text-sm text-text-dim">Browse blocks and transactions on the dev chain</p>
  </div>

  <!-- Search -->
  <div class="mb-5 flex gap-2">
    <div class="relative flex-1">
      <Search size={16} class="absolute left-3 top-1/2 -translate-y-1/2 text-text-dimmer" />
      <input
        type="text"
        placeholder="Search by block number or transaction hash..."
        bind:value={searchQuery}
        class="w-full rounded-lg border border-border bg-surface-1 py-2.5 pl-10 pr-4 text-sm text-text placeholder:text-text-dimmer focus:border-accent focus:outline-none"
      />
    </div>
  </div>

  <!-- Block table -->
  <div class="overflow-hidden rounded-xl border border-border bg-surface-1">
    <table class="w-full">
      <thead>
        <tr class="border-b border-border text-left text-xs text-text-dimmer">
          <th class="px-4 py-3 font-medium">Block</th>
          <th class="px-4 py-3 font-medium">Age</th>
          <th class="px-4 py-3 font-medium text-right">Txns</th>
          <th class="px-4 py-3 font-medium text-right">Gas Used</th>
          <th class="w-10 px-4 py-3"></th>
        </tr>
      </thead>
      <tbody>
        {#each mockBlocks as block}
          <tr class="border-b border-border/50 transition-colors last:border-0 hover:bg-surface-2">
            <td class="px-4 py-3">
              <a href="/explorer/{block.number}" class="flex items-center gap-2 text-sm font-medium text-accent hover:text-accent-hover">
                <Box size={14} class="text-text-dimmer" />
                #{block.number}
              </a>
            </td>
            <td class="px-4 py-3 text-sm text-text-dim">{block.timestamp}</td>
            <td class="px-4 py-3 text-right text-sm text-text">{block.txCount}</td>
            <td class="px-4 py-3 text-right font-mono text-sm text-text-dim">{block.gasUsed}</td>
            <td class="px-4 py-3 text-right">
              <a href="/explorer/{block.number}" class="text-text-dimmer hover:text-text">
                <ChevronRight size={16} />
              </a>
            </td>
          </tr>
        {/each}
      </tbody>
    </table>
  </div>

  <!-- Load more -->
  <div class="mt-4 flex justify-center">
    <button class="rounded-lg bg-surface-2 px-4 py-2 text-sm text-text-dim transition-colors hover:bg-surface-3 hover:text-text">
      Load More Blocks
    </button>
  </div>
</div>
</div>
