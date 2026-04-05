<script lang="ts">
  import { page } from "$app/stores";
  import { ArrowLeft, Copy, Check, ArrowRight } from "lucide-svelte";

  let copied = $state("");

  const hash = $derived($page.params.hash);
  const isBlock = $derived(!hash.startsWith("0x") || hash.length < 66);

  function copyToClipboard(text: string, key: string) {
    navigator.clipboard.writeText(text);
    copied = key;
    setTimeout(() => (copied = ""), 1500);
  }

  const mockBlock = {
    number: 5,
    hash: "0xabc123def456789abc123def456789abc123def456789abc123def456789abcd",
    parentHash: "0x999888def456789abc123def456789abc123def456789abc123def456789abcd",
    timestamp: "2025-04-05 14:32:01",
    gasUsed: "126,000",
    gasLimit: "30,000,000",
    miner: "0x71562b71999567F8C7E3BD885b2D4Fe429781573",
    txns: [
      { hash: "0xaaa111bbb222ccc333ddd444eee555fff666777888999aaa111bbb222ccc333dd", from: "0x7156...1573", to: "0xdead...beef", value: "1.5 ETH", status: "Success" },
      { hash: "0xbbb222ccc333ddd444eee555fff666777888999aaa111bbb222ccc333ddd444ee", from: "0x7156...1573", to: "0xcafe...babe", value: "0 ETH", status: "Success" },
      { hash: "0xccc333ddd444eee555fff666777888999aaa111bbb222ccc333ddd444eee555ff", from: "0x7156...1573", to: "0xface...d00d", value: "2.0 ETH", status: "Success" },
    ],
  };

  const mockTx = {
    hash: "0xaaa111bbb222ccc333ddd444eee555fff666777888999aaa111bbb222ccc333dd",
    blockNumber: 5,
    from: "0x71562b71999567F8C7E3BD885b2D4Fe429781573",
    to: "0xdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef",
    value: "1.5 ETH",
    gasUsed: "21,000",
    gasPrice: "1 wei",
    status: "Success",
    nonce: 3,
    inputData: "0x",
  };
</script>

<div class="flex-1 overflow-y-auto">
<div class="p-6">
  <a href="/explorer" class="mb-4 inline-flex items-center gap-1.5 text-sm text-text-dim transition-colors hover:text-text">
    <ArrowLeft size={14} /> Back to Explorer
  </a>

  {#if isBlock}
    <!-- Block Detail -->
    <h1 class="mb-6 text-xl font-semibold">Block #{hash}</h1>

    <div class="mb-6 rounded-xl border border-border bg-surface-1 p-5">
      <h2 class="mb-4 text-sm font-medium text-text-dim">Block Header</h2>
      <div class="grid grid-cols-1 gap-3 lg:grid-cols-2">
        {#each [
          { label: "Block Hash", value: mockBlock.hash, key: "bh" },
          { label: "Parent Hash", value: mockBlock.parentHash, key: "ph" },
          { label: "Timestamp", value: mockBlock.timestamp, key: "" },
          { label: "Gas Used / Limit", value: `${mockBlock.gasUsed} / ${mockBlock.gasLimit}`, key: "" },
          { label: "Miner", value: mockBlock.miner, key: "miner" },
        ] as field}
          <div class="flex items-start justify-between rounded-lg bg-surface-2 px-3 py-2.5">
            <div class="min-w-0 flex-1">
              <div class="text-xs text-text-dimmer">{field.label}</div>
              <div class="mt-0.5 truncate font-mono text-sm text-text">{field.value}</div>
            </div>
            {#if field.key}
              <button
                class="ml-2 shrink-0 rounded-md p-1 text-text-dim hover:text-text"
                onclick={() => copyToClipboard(field.value, field.key)}
              >
                {#if copied === field.key}
                  <Check size={12} class="text-green" />
                {:else}
                  <Copy size={12} />
                {/if}
              </button>
            {/if}
          </div>
        {/each}
      </div>
    </div>

    <!-- Transactions in block -->
    <div class="rounded-xl border border-border bg-surface-1 p-5">
      <h2 class="mb-4 text-sm font-medium text-text-dim">
        Transactions ({mockBlock.txns.length})
      </h2>
      <div class="flex flex-col gap-2">
        {#each mockBlock.txns as tx}
          <a
            href="/explorer/{tx.hash}"
            class="flex items-center justify-between rounded-lg bg-surface-2 px-4 py-3 transition-colors hover:bg-surface-3"
          >
            <div class="min-w-0 flex-1">
              <div class="truncate font-mono text-sm text-accent">{tx.hash}</div>
              <div class="mt-1 flex items-center gap-2 text-xs text-text-dim">
                <span>{tx.from}</span>
                <ArrowRight size={10} />
                <span>{tx.to}</span>
              </div>
            </div>
            <div class="ml-4 text-right">
              <div class="text-sm font-medium text-text">{tx.value}</div>
              <div class="text-xs text-green">{tx.status}</div>
            </div>
          </a>
        {/each}
      </div>
    </div>
  {:else}
    <!-- Transaction Detail -->
    <h1 class="mb-6 text-xl font-semibold">Transaction Details</h1>

    <div class="rounded-xl border border-border bg-surface-1 p-5">
      <div class="flex flex-col gap-3">
        {#each [
          { label: "Tx Hash", value: mockTx.hash, mono: true },
          { label: "Status", value: mockTx.status, mono: false },
          { label: "Block", value: String(mockTx.blockNumber), mono: false },
          { label: "From", value: mockTx.from, mono: true },
          { label: "To", value: mockTx.to, mono: true },
          { label: "Value", value: mockTx.value, mono: false },
          { label: "Gas Used", value: mockTx.gasUsed, mono: false },
          { label: "Gas Price", value: mockTx.gasPrice, mono: false },
          { label: "Nonce", value: String(mockTx.nonce), mono: false },
          { label: "Input Data", value: mockTx.inputData, mono: true },
        ] as field}
          <div class="flex items-center justify-between rounded-lg bg-surface-2 px-4 py-3">
            <div class="w-28 shrink-0 text-xs text-text-dimmer">{field.label}</div>
            <div class="min-w-0 flex-1 truncate text-right text-sm {field.mono ? 'font-mono' : ''} {field.label === 'Status' ? 'text-green' : 'text-text'}">
              {field.value}
            </div>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>
</div>
