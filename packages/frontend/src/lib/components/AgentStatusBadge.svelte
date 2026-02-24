<script lang="ts">
  import type { AgentStateType } from "@agents-dashboard/shared";

  interface Props {
    state: AgentStateType;
  }

  let { state }: Props = $props();

  const config: Record<AgentStateType, { label: string; class: string; dot: string }> = {
    running: { label: "Running", class: "bg-green-500/20 text-green-400", dot: "bg-green-400 animate-pulse" },
    idle: { label: "Idle", class: "bg-yellow-500/20 text-yellow-400", dot: "bg-yellow-400" },
    permission_waiting: { label: "Waiting", class: "bg-orange-500/20 text-orange-400", dot: "bg-orange-400 animate-pulse" },
    error: { label: "Error", class: "bg-red-500/20 text-red-400", dot: "bg-red-400" },
    stopped: { label: "Stopped", class: "bg-slate-500/20 text-slate-400", dot: "bg-slate-400" },
  };

  const c = $derived(config[state]);
</script>

<span class="inline-flex items-center gap-1.5 rounded-full px-2.5 py-0.5 text-xs font-medium {c.class}">
  <span class="h-2 w-2 rounded-full {c.dot}"></span>
  {c.label}
</span>
