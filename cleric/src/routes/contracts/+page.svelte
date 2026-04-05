<script lang="ts">
  import {
    FolderOpen,
    X,
    Rocket,
    ChevronDown,
    ChevronRight,
    Eye,
    Pencil,
    Copy,
    Check,
    FileCode,
    ArrowRight,
  } from "lucide-svelte";

  interface ConstructorArg {
    name: string;
    type: string;
    value: string;
  }

  interface ContractFn {
    name: string;
    type: string;
    inputs: { name: string; type: string }[];
  }

  interface SolFile {
    filename: string;
    contractName: string;
    constructorArgs: ConstructorArg[];
    functions: ContractFn[];
  }

  interface SelectedContract {
    order: number;
    file: SolFile;
    deployed: boolean;
    address?: string;
    txHash?: string;
    blockNumber?: number;
  }

  // Mock: discovered .sol files from a folder
  let folderPath = $state("");
  let solFiles = $state<SolFile[]>([
    {
      filename: "TETH.sol",
      contractName: "TETH",
      constructorArgs: [],
      functions: [
        { name: "name", type: "view", inputs: [] },
        { name: "symbol", type: "view", inputs: [] },
        { name: "totalSupply", type: "view", inputs: [] },
        { name: "balanceOf", type: "view", inputs: [{ name: "account", type: "address" }] },
        { name: "transfer", type: "nonpayable", inputs: [{ name: "to", type: "address" }, { name: "amount", type: "uint256" }] },
      ],
    },
    {
      filename: "Registry.sol",
      contractName: "Registry",
      constructorArgs: [
        { name: "admin", type: "address", value: "" },
      ],
      functions: [
        { name: "register", type: "nonpayable", inputs: [{ name: "name", type: "string" }] },
        { name: "lookup", type: "view", inputs: [{ name: "name", type: "string" }] },
        { name: "owner", type: "view", inputs: [] },
      ],
    },
    {
      filename: "Vault.sol",
      contractName: "Vault",
      constructorArgs: [
        { name: "token", type: "address", value: "" },
        { name: "fee", type: "uint256", value: "" },
      ],
      functions: [
        { name: "deposit", type: "nonpayable", inputs: [{ name: "amount", type: "uint256" }] },
        { name: "withdraw", type: "nonpayable", inputs: [{ name: "amount", type: "uint256" }] },
        { name: "balanceOf", type: "view", inputs: [{ name: "account", type: "address" }] },
        { name: "totalDeposits", type: "view", inputs: [] },
      ],
    },
    {
      filename: "Faucet.sol",
      contractName: "Faucet",
      constructorArgs: [
        { name: "token", type: "address", value: "" },
        { name: "dripAmount", type: "uint256", value: "" },
        { name: "cooldown", type: "uint256", value: "" },
      ],
      functions: [
        { name: "drip", type: "nonpayable", inputs: [{ name: "to", type: "address" }] },
        { name: "setDripAmount", type: "nonpayable", inputs: [{ name: "amount", type: "uint256" }] },
        { name: "lastDrip", type: "view", inputs: [{ name: "account", type: "address" }] },
      ],
    },
    {
      filename: "Migrations.sol",
      contractName: "Migrations",
      constructorArgs: [],
      functions: [
        { name: "last_completed_migration", type: "view", inputs: [] },
        { name: "setCompleted", type: "nonpayable", inputs: [{ name: "completed", type: "uint256" }] },
      ],
    },
  ]);

  let selectedContracts = $state<SelectedContract[]>([]);
  let expandedDeploy = $state<Record<string, boolean>>({});
  let copied = $state("");

  // Mock: one already deployed to show the post-deploy UI
  $effect(() => {
    if (selectedContracts.length === 0 && solFiles.length > 0) {
      // Pre-select first two as demo
    }
  });

  function isSelected(contractName: string): number | null {
    const idx = selectedContracts.findIndex((s) => s.file.contractName === contractName);
    return idx >= 0 ? idx + 1 : null;
  }

  function toggleSelect(file: SolFile) {
    const existing = selectedContracts.findIndex((s) => s.file.contractName === file.contractName);
    if (existing >= 0) {
      // Remove and re-number
      selectedContracts = selectedContracts
        .filter((_, i) => i !== existing)
        .map((s, i) => ({ ...s, order: i + 1 }));
    } else {
      // Add with next order number
      selectedContracts = [
        ...selectedContracts,
        {
          order: selectedContracts.length + 1,
          file,
          deployed: false,
        },
      ];
    }
  }

  function removeSelected(index: number) {
    selectedContracts = selectedContracts
      .filter((_, i) => i !== index)
      .map((s, i) => ({ ...s, order: i + 1 }));
  }

  function mockDeploy(index: number) {
    selectedContracts[index] = {
      ...selectedContracts[index],
      deployed: true,
      address: "0x" + Array.from({ length: 40 }, () => "0123456789abcdef"[Math.floor(Math.random() * 16)]).join(""),
      txHash: "0x" + Array.from({ length: 64 }, () => "0123456789abcdef"[Math.floor(Math.random() * 16)]).join(""),
      blockNumber: Math.floor(Math.random() * 100) + 1,
    };
  }

  function copyText(text: string, key: string) {
    navigator.clipboard.writeText(text);
    copied = key;
    setTimeout(() => (copied = ""), 1500);
  }
</script>

<div class="flex min-h-0 flex-1 flex-col p-6">
  <div class="mb-6">
    <h1 class="text-xl font-semibold text-text">Contracts</h1>
    <p class="text-sm text-text-dim">Browse Solidity sources, pick deploy order, and deploy</p>
  </div>

  <!-- Folder picker -->
  <div class="mb-5 flex items-center gap-3">
    <button
      class="flex items-center gap-2 rounded-lg bg-accent px-4 py-2.5 text-sm font-medium text-white transition-colors hover:bg-accent-hover"
    >
      <FolderOpen size={15} /> Browse Folder
    </button>
    {#if solFiles.length > 0}
      <div class="flex items-center gap-2 rounded-lg bg-surface-1 border border-border px-3 py-2 text-sm">
        <FileCode size={14} class="text-text-dimmer" />
        <span class="font-mono text-text-dim">contracts/src/</span>
        <span class="text-text-dimmer">&middot; {solFiles.length} files found</span>
      </div>
    {/if}
  </div>

  {#if solFiles.length > 0}
    <div class="flex flex-1 gap-5 min-h-0">
      <!-- LEFT: Source files as selectable tags -->
      <div class="w-72 shrink-0 flex flex-col">
        <h2 class="mb-3 text-xs font-medium uppercase tracking-wider text-text-dimmer">Source Files</h2>
        <div class="flex-1 overflow-y-auto rounded-xl border border-border bg-surface-1 p-3">
          <div class="flex flex-col gap-1.5">
            {#each solFiles as file}
              {@const order = isSelected(file.contractName)}
              <button
                class="group flex items-center gap-2.5 rounded-lg px-3 py-2.5 text-left transition-all
                  {order
                    ? 'bg-accent/15 border border-accent/30'
                    : 'bg-surface-2 border border-transparent hover:bg-surface-3 hover:border-border-hover'}"
                onclick={() => toggleSelect(file)}
              >
                <!-- Order badge or empty slot -->
                {#if order}
                  <div class="flex h-6 w-6 items-center justify-center rounded-md bg-accent text-xs font-bold text-white">
                    {order}
                  </div>
                {:else}
                  <div class="flex h-6 w-6 items-center justify-center rounded-md bg-surface-3 text-xs text-text-dimmer group-hover:bg-border">
                    +
                  </div>
                {/if}

                <div class="flex-1 min-w-0">
                  <div class="text-sm font-medium {order ? 'text-accent-hover' : 'text-text'}">{file.contractName}</div>
                  <div class="text-xs text-text-dimmer truncate">{file.filename}</div>
                </div>

                {#if file.constructorArgs.length > 0}
                  <span class="rounded-full bg-surface-3 px-1.5 py-0.5 text-[10px] text-text-dimmer">
                    {file.constructorArgs.length} arg{file.constructorArgs.length > 1 ? "s" : ""}
                  </span>
                {/if}
              </button>
            {/each}
          </div>
        </div>
        <div class="mt-2 text-xs text-text-dimmer">Click to select &middot; Order = deploy sequence</div>
      </div>

      <!-- RIGHT: Deploy queue -->
      <div class="flex-1 flex flex-col min-w-0">
        <div class="mb-3 flex items-center justify-between">
          <h2 class="text-xs font-medium uppercase tracking-wider text-text-dimmer">
            Deploy Queue ({selectedContracts.length})
          </h2>
          {#if selectedContracts.length > 0 && !selectedContracts.every((s) => s.deployed)}
            <button class="flex items-center gap-2 rounded-lg bg-green px-4 py-2 text-sm font-medium text-white transition-opacity hover:opacity-90">
              <Rocket size={14} /> Deploy All
            </button>
          {/if}
        </div>

        <div class="flex-1 overflow-y-auto">
          {#if selectedContracts.length === 0}
            <div class="flex h-full flex-col items-center justify-center rounded-xl border-2 border-dashed border-border py-16 text-center">
              <ArrowRight size={32} class="mb-3 text-text-dimmer -rotate-180" />
              <p class="text-sm text-text-dim">Select contracts from the left</p>
              <p class="text-xs text-text-dimmer">Click in the order you want them deployed</p>
            </div>
          {:else}
            <div class="flex flex-col gap-3">
              {#each selectedContracts as sc, i}
                <div class="rounded-xl border bg-surface-1 {sc.deployed ? 'border-green/30' : 'border-border'}">
                  <!-- Header -->
                  <div class="flex items-center gap-3 px-4 py-3">
                    <div class="flex h-7 w-7 items-center justify-center rounded-md {sc.deployed ? 'bg-green/15 text-green' : 'bg-accent/15 text-accent'} text-xs font-bold">
                      {sc.order}
                    </div>
                    <div class="flex-1 min-w-0">
                      <div class="flex items-center gap-2">
                        <span class="text-sm font-semibold text-text">{sc.file.contractName}</span>
                        <span class="font-mono text-xs text-text-dimmer">{sc.file.filename}</span>
                        {#if sc.deployed}
                          <span class="rounded-full bg-green/15 px-2 py-0.5 text-xs font-medium text-green">Deployed</span>
                        {/if}
                      </div>
                    </div>
                    <div class="flex items-center gap-1">
                      {#if !sc.deployed}
                        <button
                          class="flex items-center gap-1.5 rounded-lg bg-green/15 px-3 py-1.5 text-xs font-medium text-green transition-colors hover:bg-green/25"
                          onclick={() => mockDeploy(i)}
                        >
                          <Rocket size={12} /> Deploy
                        </button>
                      {/if}
                      <button
                        class="rounded-md p-1.5 text-text-dim transition-colors hover:bg-surface-2 hover:text-text"
                        onclick={() => (expandedDeploy[sc.file.contractName] = !expandedDeploy[sc.file.contractName])}
                      >
                        {#if expandedDeploy[sc.file.contractName]}
                          <ChevronDown size={16} />
                        {:else}
                          <ChevronRight size={16} />
                        {/if}
                      </button>
                      <button
                        class="rounded-md p-1.5 text-text-dim transition-colors hover:bg-red/15 hover:text-red"
                        onclick={() => removeSelected(i)}
                      >
                        <X size={14} />
                      </button>
                    </div>
                  </div>

                  <!-- Constructor args (always visible if not deployed and has args) -->
                  {#if sc.file.constructorArgs.length > 0 && !sc.deployed}
                    <div class="border-t border-border px-4 py-3">
                      <h3 class="mb-2 text-xs font-medium text-text-dim">Constructor Arguments</h3>
                      <div class="flex flex-col gap-2">
                        {#each sc.file.constructorArgs as arg}
                          <div class="flex items-center gap-2">
                            <span class="w-28 shrink-0 text-xs text-text-dimmer">
                              {arg.name}
                              <span class="opacity-60">({arg.type})</span>
                            </span>
                            <input
                              type="text"
                              placeholder={arg.type}
                              bind:value={arg.value}
                              class="flex-1 rounded-lg border border-border bg-surface-2 px-3 py-1.5 text-sm font-mono text-text placeholder:text-text-dimmer focus:border-accent focus:outline-none"
                            />
                          </div>
                        {/each}
                      </div>
                    </div>
                  {/if}

                  <!-- Deployed details -->
                  {#if sc.deployed}
                    <div class="border-t border-border px-4 py-3">
                      <div class="flex flex-col gap-2">
                        <div class="flex items-center justify-between rounded-lg bg-green/5 px-3 py-2">
                          <div>
                            <div class="text-xs text-text-dimmer">Contract Address</div>
                            <div class="font-mono text-sm text-green">{sc.address}</div>
                          </div>
                          <button
                            class="rounded-md p-1.5 text-text-dim hover:text-text"
                            onclick={() => copyText(sc.address ?? "", `addr-${i}`)}
                          >
                            {#if copied === `addr-${i}`}
                              <Check size={12} class="text-green" />
                            {:else}
                              <Copy size={12} />
                            {/if}
                          </button>
                        </div>
                        <div class="flex gap-3 text-xs">
                          <div class="rounded-lg bg-surface-2 px-3 py-2 flex-1">
                            <span class="text-text-dimmer">Tx Hash: </span>
                            <span class="font-mono text-text truncate">{sc.txHash?.slice(0, 18)}...</span>
                          </div>
                          <div class="rounded-lg bg-surface-2 px-3 py-2">
                            <span class="text-text-dimmer">Block: </span>
                            <span class="font-mono text-text">#{sc.blockNumber}</span>
                          </div>
                        </div>
                      </div>
                    </div>
                  {/if}

                  <!-- Expanded: functions -->
                  {#if expandedDeploy[sc.file.contractName]}
                    <div class="border-t border-border px-4 py-3">
                      <h3 class="mb-2 text-xs font-medium text-text-dim">Functions ({sc.file.functions.length})</h3>
                      <div class="flex flex-col gap-1.5">
                        {#each sc.file.functions as fn}
                          <div class="flex items-center gap-3 rounded-lg bg-surface-2 px-3 py-2">
                            <div class="flex h-5 w-5 items-center justify-center rounded {fn.type === 'view' ? 'bg-blue-500/15 text-blue-400' : 'bg-orange/15 text-orange'}">
                              {#if fn.type === "view"}
                                <Eye size={10} />
                              {:else}
                                <Pencil size={10} />
                              {/if}
                            </div>
                            <span class="text-sm text-text">{fn.name}</span>
                            <span class="text-xs text-text-dimmer">
                              ({fn.inputs.map((inp) => `${inp.type}`).join(", ")})
                            </span>
                            <span class="ml-auto rounded-full px-2 py-0.5 text-[10px] {fn.type === 'view' ? 'bg-blue-500/10 text-blue-400' : 'bg-orange/10 text-orange'}">
                              {fn.type}
                            </span>
                          </div>
                        {/each}
                      </div>
                    </div>
                  {/if}
                </div>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    </div>
  {:else}
    <!-- Empty state: no folder loaded -->
    <div class="flex flex-1 flex-col items-center justify-center rounded-xl border-2 border-dashed border-border py-20 text-center">
      <FolderOpen size={48} class="mb-3 text-text-dimmer" />
      <p class="text-sm text-text-dim">No project folder loaded</p>
      <p class="text-xs text-text-dimmer">Browse a folder containing Solidity source files to get started</p>
    </div>
  {/if}
</div>
