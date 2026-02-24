<script lang="ts">
  import { agentsStore } from "$lib/stores/agents.svelte.js";
  import type { RecencyFilter } from "$lib/stores/agents.svelte.js";
  import AgentCard from "$lib/components/AgentCard.svelte";
  import SearchBar from "$lib/components/SearchBar.svelte";
  import type { AgentStateType } from "@agents-dashboard/shared";

  const recencyMs: Record<RecencyFilter, number> = {
    "1h": 60 * 60 * 1000,
    "6h": 6 * 60 * 60 * 1000,
    "24h": 24 * 60 * 60 * 1000,
    "7d": 7 * 24 * 60 * 60 * 1000,
    "30d": 30 * 24 * 60 * 60 * 1000,
    all: 0,
  };

  const filteredSessions = $derived.by(() => {
    const filter = agentsStore.recencyFilter;
    if (filter === "all") return agentsStore.sessions;
    const cutoff = Date.now() - recencyMs[filter];
    return agentsStore.sessions.filter(
      (s) => new Date(s.lastActivityAt).getTime() >= cutoff,
    );
  });

  // Sort: running first, then permission_waiting, idle, error, stopped
  const stateOrder: Record<AgentStateType, number> = {
    running: 0,
    permission_waiting: 1,
    idle: 2,
    error: 3,
    stopped: 4,
  };

  const sortedSessions = $derived(
    [...filteredSessions].sort((a, b) => {
      const stateA = stateOrder[a.state] ?? 5;
      const stateB = stateOrder[b.state] ?? 5;
      if (stateA !== stateB) return stateA - stateB;
      return new Date(b.lastActivityAt).getTime() - new Date(a.lastActivityAt).getTime();
    }),
  );

  const isSearchActive = $derived(!!agentsStore.searchResults);

  const displaySessions = $derived(
    isSearchActive
      ? agentsStore.searchResults!.results.map((r) => r.session)
      : sortedSessions,
  );

  const activeSessions = $derived(
    filteredSessions.filter((s) => s.state !== "stopped"),
  );

  const totalCost = $derived(
    filteredSessions.reduce((sum, s) => sum + s.cumulativeUsage.estimatedCost, 0),
  );
</script>

<div class="space-y-6">
  <!-- Search bar -->
  <SearchBar />

  <!-- Summary bar -->
  <div class="flex items-center gap-6 text-sm text-slate-400">
    {#if isSearchActive}
      <span>{agentsStore.searchResults!.totalSessions} results</span>
    {:else}
      <span>{filteredSessions.length} sessions</span>
      <span>{activeSessions.length} active</span>
    {/if}
    <span>Total cost: ${totalCost.toFixed(4)}</span>
    {#if !isSearchActive}
      <select
        class="ml-auto rounded bg-slate-800 px-2 py-1 text-sm text-slate-300 border border-slate-700 focus:outline-none focus:border-slate-500"
        value={agentsStore.recencyFilter}
        onchange={(e) => agentsStore.setRecencyFilter(e.currentTarget.value as RecencyFilter)}
      >
        <option value="1h">Last 1 hour</option>
        <option value="6h">Last 6 hours</option>
        <option value="24h">Last 24 hours</option>
        <option value="7d">Last 7 days</option>
        <option value="30d">Last 30 days</option>
        <option value="all">All time</option>
      </select>
    {/if}
  </div>

  <!-- Agent grid -->
  {#if displaySessions.length === 0}
    <div class="flex flex-col items-center justify-center py-20 text-slate-500">
      {#if isSearchActive}
        <p class="text-lg">No sessions match "{agentsStore.searchQuery}"</p>
        <p class="mt-2 text-sm">Try a different keyword or adjust the search scope.</p>
      {:else if agentsStore.recencyFilter !== "all"}
        <p class="text-lg">No sessions in this time range</p>
        <p class="mt-2 text-sm">Try a longer time range or select "All time".</p>
      {:else}
        <p class="text-lg">No agent sessions detected</p>
        <p class="mt-2 text-sm">Start a Claude Code session and it will appear here.</p>
      {/if}
    </div>
  {:else}
    <div class="grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3">
      {#each displaySessions as session (session.sessionId)}
        <AgentCard {session} searchResult={agentsStore.getSearchResult(session.sessionId)} />
      {/each}
    </div>
  {/if}
</div>
