<script lang="ts">
  import "../app.css";
  import { page } from "$app/stores";
  import { fade } from "svelte/transition";
  import {
    LayoutDashboard,
    Search,
    FileCode2,
    ScrollText,
    Settings,
  } from "lucide-svelte";

  let { children } = $props();

  const tabs = [
    { href: "/dashboard", label: "Dashboard", icon: LayoutDashboard },
    { href: "/explorer", label: "Explorer", icon: Search },
    { href: "/contracts", label: "Contracts", icon: FileCode2 },
    { href: "/logs", label: "Logs", icon: ScrollText },
    { href: "/settings", label: "Settings", icon: Settings },
  ];

  let druidRunning = $state(false);
</script>

<div class="flex h-screen w-screen overflow-hidden">
  <!-- Sidebar -->
  <nav class="glass-panel z-10 flex w-56 shrink-0 flex-col border-r-0 border-r border-[#2a2a3a]">
    <!-- Logo / Title -->
    <div class="flex items-center gap-3 border-b border-border px-5 py-5">
      <div class="flex h-10 w-10 items-center justify-center rounded-xl bg-gradient-to-br from-[#8a2be2] to-[#00f0ff] font-bold text-white shadow-[0_0_15px_rgba(138,43,226,0.3)]">
        <span class="drop-shadow-md">C</span>
      </div>
      <div>
        <div class="text-base font-bold tracking-wide text-white">Cleric</div>
        <div class="text-[0.65rem] font-medium uppercase tracking-widest text-[#00f0ff]">Druid Manager</div>
      </div>
    </div>

    <!-- Nav tabs -->
    <div class="flex flex-1 flex-col gap-1.5 px-3 py-4">
      {#each tabs as tab}
        {@const active = $page.url.pathname.startsWith(tab.href)}
        <a
          href={tab.href}
          class="group relative flex items-center gap-3 rounded-xl px-3 py-2.5 text-sm transition-all duration-200
            {active
            ? 'bg-[#00f0ff]/10 font-semibold text-white shadow-[inset_0_0_10px_rgba(0,240,255,0.05)]'
            : 'text-text-dim hover:bg-surface-2 hover:text-white'}"
        >
          {#if active}
            <div class="absolute left-0 top-1/2 h-1/2 w-1 -translate-y-1/2 rounded-r bg-[#00f0ff] shadow-[0_0_10px_#00f0ff]" transition:fade={{duration: 150}}></div>
          {/if}
          <div class="transition-transform group-hover:scale-110">
            <tab.icon size={18} color={active ? "#00f0ff" : "currentColor"} />
          </div>
          {tab.label}
        </a>
      {/each}
    </div>

    <!-- Status footer -->
    <div class="border-t border-border px-5 py-4">
      <div class="flex items-center gap-3">
        <div class="relative flex h-3 w-3">
          {#if druidRunning}
            <span class="absolute inline-flex h-full w-full animate-ping rounded-full bg-[#00ff66] opacity-40"></span>
          {/if}
          <span class="relative inline-flex h-3 w-3 rounded-full {druidRunning ? 'bg-[#00ff66]' : 'bg-[#ff0055]'} shadow-[0_0_8px_currentColor]"></span>
        </div>
        <div class="flex flex-col">
          <span class="text-xs font-semibold text-white">
            {druidRunning ? "DRUID ONLINE" : "DRUID OFFLINE"}
          </span>
          <span class="mt-0.5 font-mono text-[0.65rem] text-text-dimmer">ID: 1337 • DEV</span>
        </div>
      </div>
    </div>
  </nav>

  <!-- Main content -->
  <main class="relative flex flex-1 flex-col overflow-y-auto overflow-x-hidden">
    {@render children()}
  </main>
</div>
