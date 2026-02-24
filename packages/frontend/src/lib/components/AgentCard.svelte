<script lang="ts">
  import type { AgentSessionSummary, SessionSearchResult } from "@agents-dashboard/shared";
  import AgentStatusBadge from "./AgentStatusBadge.svelte";

  interface Props {
    session: AgentSessionSummary;
    searchResult?: SessionSearchResult;
  }

  let { session, searchResult }: Props = $props();

  function formatCost(cost: number): string {
    return "$" + cost.toFixed(4);
  }

  function formatTokens(n: number): string {
    if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
    if (n >= 1_000) return (n / 1_000).toFixed(1) + "K";
    return n.toString();
  }

  function timeAgo(isoStr: string): string {
    const diff = Date.now() - new Date(isoStr).getTime();
    const secs = Math.floor(diff / 1000);
    if (secs < 60) return `${secs}s ago`;
    const mins = Math.floor(secs / 60);
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  }

  const totalTokens = $derived(
    session.cumulativeUsage.inputTokens +
    session.cumulativeUsage.outputTokens +
    session.cumulativeUsage.cacheReadTokens
  );

  const previewMatches = $derived(searchResult?.matches.slice(0, 2) ?? []);
</script>

<a
  href="/agent/{session.sessionId}"
  class="block rounded-lg border border-slate-700 bg-slate-800 p-4 transition-colors hover:border-slate-600 hover:bg-slate-800/80"
>
  <div class="flex items-start justify-between">
    <div class="min-w-0 flex-1">
      <div class="flex items-center gap-2">
        <h3 class="truncate text-sm font-semibold text-slate-100">{session.projectName}</h3>
        <AgentStatusBadge state={session.state} />
        {#if searchResult}
          <span class="rounded-full bg-blue-500/20 px-2 py-0.5 text-xs text-blue-300">
            {searchResult.matchCount} {searchResult.matchCount === 1 ? "match" : "matches"}
          </span>
        {/if}
      </div>
      <p class="mt-1 truncate text-xs text-slate-400">{session.model}</p>
    </div>
    <span class="ml-2 text-xs text-slate-500">{timeAgo(session.lastActivityAt)}</span>
  </div>

  {#if session.currentTask}
    <p class="mt-2 line-clamp-2 text-xs text-slate-300">{session.currentTask}</p>
  {/if}

  {#if previewMatches.length > 0}
    <div class="mt-2 space-y-1">
      {#each previewMatches as match}
        <div class="rounded bg-slate-900/50 px-2 py-1 text-xs text-slate-400">
          <span class="mr-1 text-slate-500">[{match.scope.replace("_", " ")}]</span>
          <span class="text-slate-300">{match.content}</span>
        </div>
      {/each}
    </div>
  {/if}

  <div class="mt-3 flex items-center gap-4 text-xs text-slate-400">
    <span title="Total tokens">{formatTokens(totalTokens)} tokens</span>
    <span title="Estimated cost">{formatCost(session.cumulativeUsage.estimatedCost)}</span>
    {#if session.gitStatus?.branch}
      <span class="flex items-center gap-1 truncate" title={session.gitStatus.branch}>
        <svg class="h-3 w-3 shrink-0 text-slate-500" viewBox="0 0 16 16" fill="currentColor">
          <path d="M9.5 3.25a2.25 2.25 0 1 1 3 2.122V6A2.5 2.5 0 0 1 10 8.5H6a1 1 0 0 0-1 1v1.128a2.251 2.251 0 1 1-1.5 0V5.372a2.25 2.25 0 1 1 1.5 0v1.836A2.5 2.5 0 0 1 6 7h4a1 1 0 0 0 1-1v-.628A2.25 2.25 0 0 1 9.5 3.25Zm-6 0a.75.75 0 1 0 1.5 0 .75.75 0 0 0-1.5 0Zm8.25-.75a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5ZM4.25 12a.75.75 0 1 0 0 1.5.75.75 0 0 0 0-1.5Z" />
        </svg>
        <span class="max-w-[8rem] truncate">{session.gitStatus.branch}</span>
        {#if session.gitStatus.additions > 0}
          <span class="text-green-400">+{session.gitStatus.additions}</span>
        {/if}
        {#if session.gitStatus.deletions > 0}
          <span class="text-red-400">-{session.gitStatus.deletions}</span>
        {/if}
      </span>
    {/if}
    <span class="ml-auto truncate text-slate-500" title={session.workingDirectory}>
      {session.sessionId.slice(0, 8)}
    </span>
  </div>
</a>
