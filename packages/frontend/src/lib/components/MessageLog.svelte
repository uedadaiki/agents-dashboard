<script lang="ts">
  import type { AgentMessage } from "@agents-dashboard/shared";
  import MessageEntry from "./MessageEntry.svelte";
  import { tick } from "svelte";

  interface Props {
    messages: AgentMessage[];
  }

  let { messages }: Props = $props();

  let container: HTMLDivElement | undefined = $state();
  let autoScroll = $state(true);

  function handleScroll() {
    if (!container) return;
    const { scrollTop, scrollHeight, clientHeight } = container;
    autoScroll = scrollHeight - scrollTop - clientHeight < 50;
  }

  $effect(() => {
    // eslint-disable-next-line @typescript-eslint/no-unused-expressions
    messages.length;
    if (autoScroll && container) {
      tick().then(() => {
        container!.scrollTop = container!.scrollHeight;
      });
    }
  });
</script>

<div
  bind:this={container}
  onscroll={handleScroll}
  class="flex-1 overflow-y-auto rounded-lg border border-slate-700 bg-slate-900 p-4"
>
  {#if messages.length === 0}
    <p class="text-center text-sm text-slate-500">No messages yet. Subscribe to see activity.</p>
  {:else}
    <div class="divide-y divide-slate-800">
      {#each messages as message (message.id)}
        <MessageEntry {message} />
      {/each}
    </div>
  {/if}
</div>
