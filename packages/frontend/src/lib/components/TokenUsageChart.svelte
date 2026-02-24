<script lang="ts">
  import type { CumulativeUsage } from "@agents-dashboard/shared";

  interface Props {
    usage: CumulativeUsage;
  }

  let { usage }: Props = $props();

  function formatTokens(n: number): string {
    if (n >= 1_000_000) return (n / 1_000_000).toFixed(1) + "M";
    if (n >= 1_000) return (n / 1_000).toFixed(1) + "K";
    return n.toString();
  }

  const total = $derived(
    usage.inputTokens + usage.outputTokens + usage.cacheReadTokens + usage.cacheCreationTokens,
  );

  function pct(n: number): string {
    if (total === 0) return "0";
    return ((n / total) * 100).toFixed(0);
  }
</script>

<div class="rounded-lg border border-slate-700 bg-slate-900 p-4">
  <h3 class="mb-3 text-sm font-medium text-slate-300">Token Usage</h3>

  <!-- Stacked bar -->
  <div class="mb-3 flex h-3 overflow-hidden rounded-full bg-slate-800">
    {#if total > 0}
      <div class="bg-blue-500" style="width: {pct(usage.inputTokens)}%" title="Input"></div>
      <div class="bg-green-500" style="width: {pct(usage.outputTokens)}%" title="Output"></div>
      <div class="bg-purple-500" style="width: {pct(usage.cacheReadTokens)}%" title="Cache read"></div>
      <div class="bg-orange-500" style="width: {pct(usage.cacheCreationTokens)}%" title="Cache creation"></div>
    {/if}
  </div>

  <!-- Legend -->
  <div class="grid grid-cols-2 gap-2 text-xs">
    <div class="flex items-center gap-2">
      <span class="h-2 w-2 rounded-full bg-blue-500"></span>
      <span class="text-slate-400">Input:</span>
      <span class="text-slate-200">{formatTokens(usage.inputTokens)}</span>
    </div>
    <div class="flex items-center gap-2">
      <span class="h-2 w-2 rounded-full bg-green-500"></span>
      <span class="text-slate-400">Output:</span>
      <span class="text-slate-200">{formatTokens(usage.outputTokens)}</span>
    </div>
    <div class="flex items-center gap-2">
      <span class="h-2 w-2 rounded-full bg-purple-500"></span>
      <span class="text-slate-400">Cache read:</span>
      <span class="text-slate-200">{formatTokens(usage.cacheReadTokens)}</span>
    </div>
    <div class="flex items-center gap-2">
      <span class="h-2 w-2 rounded-full bg-orange-500"></span>
      <span class="text-slate-400">Cache create:</span>
      <span class="text-slate-200">{formatTokens(usage.cacheCreationTokens)}</span>
    </div>
  </div>

  <div class="mt-3 border-t border-slate-700 pt-2 text-right text-sm">
    <span class="text-slate-400">Est. cost: </span>
    <span class="font-medium text-slate-100">${usage.estimatedCost.toFixed(4)}</span>
  </div>
</div>
