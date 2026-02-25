<script lang="ts">
  import "../app.css";
  import { wsStore } from "$lib/stores/websocket.svelte.js";
  import { agentsStore } from "$lib/stores/agents.svelte.js";
  import { isMockMode, mockSessions, mockSessionMessages } from "$lib/mock-data.js";
  import NotificationSettings from "$lib/components/NotificationSettings.svelte";

  let { children } = $props();

  if (isMockMode()) {
    agentsStore.sessions = mockSessions;
    agentsStore.sessionMessages = mockSessionMessages;
    wsStore.connected = true;
  }
</script>

<div class="min-h-screen">
  <header class="border-b border-slate-700 bg-slate-900/50 px-6 py-3">
    <div class="flex items-center justify-between">
      <a
        href="/"
        class="flex items-center gap-2 text-sm font-bold text-slate-100"
      >
        Agents Dashboard
      </a>
      <div class="flex items-center gap-3">
        <NotificationSettings />
        {#if wsStore.connected}
          <span class="h-2 w-2 rounded-full bg-green-400"></span>
          <span class="text-xs text-slate-400">Connected</span>
        {:else}
          <span class="h-2 w-2 rounded-full bg-red-400 animate-pulse"></span>
          <span class="text-xs text-slate-400">Disconnected</span>
        {/if}
      </div>
    </div>
  </header>

  <main class="mx-auto max-w-7xl px-6 py-6">
    {@render children()}
  </main>
</div>
