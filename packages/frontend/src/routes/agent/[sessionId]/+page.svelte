<script lang="ts">
  import { page } from "$app/state";
  import { agentsStore } from "$lib/stores/agents.svelte.js";
  import { wsStore } from "$lib/stores/websocket.svelte.js";
  import { isMockMode } from "$lib/mock-data.js";
  import AgentStatusBadge from "$lib/components/AgentStatusBadge.svelte";
  import MessageLog from "$lib/components/MessageLog.svelte";
  import TokenUsageChart from "$lib/components/TokenUsageChart.svelte";
  const sessionId = $derived(page.params.sessionId);
  const session = $derived(sessionId ? agentsStore.getSession(sessionId) : undefined);
  const messages = $derived(sessionId ? agentsStore.getMessages(sessionId) : []);

  $effect(() => {
    if (!sessionId || isMockMode()) return;
    agentsStore.fetchMessages(sessionId);
    wsStore.subscribe(sessionId);
    return () => wsStore.unsubscribeSession(sessionId);
  });
</script>

<div class="space-y-4">
  <!-- Back link -->
  <a href="/" class="inline-flex items-center gap-1 text-sm text-slate-400 hover:text-slate-200">
    &larr; Back to dashboard
  </a>

  {#if session}
    <!-- Header -->
    <div class="flex items-start justify-between">
      <div>
        <div class="flex items-center gap-3">
          <h1 class="text-xl font-bold text-slate-100">{session.projectName}</h1>
          <AgentStatusBadge state={session.state} />
        </div>
        <p class="mt-1 text-sm text-slate-400">{session.model}</p>
        <p class="mt-0.5 text-xs text-slate-500">{session.workingDirectory}</p>
        {#if session.gitStatus?.branch}
          <p class="mt-0.5 flex items-center gap-1 text-xs text-slate-400">
            <svg class="h-3 w-3 shrink-0 text-slate-500" viewBox="0 0 16 16" fill="currentColor">
              <path d="M9.5 3.25a2.25 2.25 0 1 1 3 2.122V6A2.5 2.5 0 0 1 10 8.5H6a1 1 0 0 0-1 1v1.128a2.251 2.251 0 1 1-1.5 0V5.372a2.25 2.25 0 1 1 1.5 0v1.836A2.5 2.5 0 0 1 6 7h4a1 1 0 0 0 1-1v-.628A2.25 2.25 0 0 1 9.5 3.25Zm-6 0a.75.75 0 1 0 1.5 0 .75.75 0 0 0-1.5 0Zm8.25-.75a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5ZM4.25 12a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Z" />
            </svg>
            <span>{session.gitStatus.branch}</span>
            {#if session.gitStatus.additions > 0}
              <span class="text-green-400">+{session.gitStatus.additions}</span>
            {/if}
            {#if session.gitStatus.deletions > 0}
              <span class="text-red-400">-{session.gitStatus.deletions}</span>
            {/if}
          </p>
        {/if}
      </div>
      <div class="text-right text-xs text-slate-500">
        <p>Session: {session.sessionId.slice(0, 8)}</p>
        <p>Provider: {session.provider}</p>
      </div>
    </div>

    {#if session.currentTask}
      <div class="rounded-lg border border-slate-700 bg-slate-900 p-3">
        <h3 class="mb-1 text-xs font-medium text-slate-400">Current Task</h3>
        <p class="text-sm text-slate-200">{session.currentTask}</p>
      </div>
    {/if}

    <div class="grid grid-cols-1 gap-4 lg:grid-cols-3">
      <!-- Token usage -->
      <div class="lg:col-span-1">
        <TokenUsageChart usage={session.cumulativeUsage} />
      </div>

      <!-- Message log -->
      <div class="flex h-[calc(100vh-22rem)] flex-col lg:col-span-2">
        <h3 class="mb-2 text-sm font-medium text-slate-300">Activity Log</h3>
        <MessageLog {messages} />
      </div>
    </div>
  {:else}
    <div class="py-20 text-center text-slate-500">
      <p class="text-lg">Session not found</p>
      <p class="mt-2 text-sm">Session {sessionId} may have ended or not been discovered yet.</p>
    </div>
  {/if}
</div>
