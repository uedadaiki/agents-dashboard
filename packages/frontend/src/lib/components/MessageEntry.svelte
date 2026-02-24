<script lang="ts">
  import type { AgentMessage } from "@agents-dashboard/shared";

  interface Props {
    message: AgentMessage;
  }

  let { message }: Props = $props();

  function formatTime(iso: string): string {
    return new Date(iso).toLocaleTimeString();
  }

  const roleColors: Record<string, string> = {
    user: "text-blue-400",
    assistant: "text-green-400",
    system: "text-yellow-400",
  };

  const typeIcons: Record<string, string> = {
    text: "",
    tool_use: "\u{1F527}",
    tool_result: "\u{1F4E6}",
    thinking: "\u{1F4AD}",
    state_change: "\u{1F504}",
    error: "\u{26A0}",
  };
</script>

<div class="flex gap-3 py-2 text-sm">
  <span class="w-16 shrink-0 text-xs text-slate-500">{formatTime(message.timestamp)}</span>
  <span class="w-5 shrink-0 text-center">{typeIcons[message.type] ?? ""}</span>
  <div class="min-w-0 flex-1">
    <span class="text-xs font-medium {roleColors[message.role] ?? 'text-slate-400'}">
      {message.role}
    </span>
    {#if message.type === "tool_use"}
      <span class="ml-2 rounded bg-slate-700 px-1.5 py-0.5 text-xs text-slate-300">
        {message.content}
      </span>
    {:else if message.type === "tool_result"}
      <p class="mt-0.5 whitespace-pre-wrap break-all text-xs text-slate-400">{message.content}</p>
    {:else if message.type === "state_change"}
      <span class="ml-2 text-xs text-yellow-400">{message.content}</span>
    {:else}
      <p class="mt-0.5 whitespace-pre-wrap break-words text-xs text-slate-300">{message.content}</p>
    {/if}
  </div>
</div>
