<script lang="ts">
  import { agentsStore } from "$lib/stores/agents.svelte.js";
  import type { SearchScope } from "@agents-dashboard/shared";

  let debounceTimer: ReturnType<typeof setTimeout> | null = null;
  let dropdownOpen = $state(false);
  let dropdownRef = $state<HTMLDivElement | null>(null);

  const scopeLabels: Record<SearchScope, string> = {
    project_name: "Project",
    current_task: "Task",
    working_directory: "Directory",
    content: "Messages",
  };

  function handleInput(e: Event) {
    const value = (e.target as HTMLInputElement).value;
    if (debounceTimer) clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
      agentsStore.search(value);
    }, 300);
  }

  function handleKeydown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      agentsStore.clearSearch();
      (e.target as HTMLInputElement).value = "";
    }
  }

  function handleClickOutside(e: MouseEvent) {
    if (dropdownRef && !dropdownRef.contains(e.target as Node)) {
      dropdownOpen = false;
    }
  }

  function toggleScope(scope: SearchScope) {
    agentsStore.toggleScope(scope);
  }

  $effect(() => {
    if (dropdownOpen) {
      document.addEventListener("click", handleClickOutside, true);
    } else {
      document.removeEventListener("click", handleClickOutside, true);
    }
    return () => document.removeEventListener("click", handleClickOutside, true);
  });

  const activeCount = $derived(agentsStore.searchScopes.length);
</script>

<div class="relative flex items-center gap-2">
  <div class="relative flex-1">
    <!-- Search icon -->
    <svg
      class="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-slate-400"
      fill="none"
      stroke="currentColor"
      viewBox="0 0 24 24"
    >
      <path
        stroke-linecap="round"
        stroke-linejoin="round"
        stroke-width="2"
        d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
      />
    </svg>

    <input
      type="text"
      placeholder="Search sessions..."
      value={agentsStore.searchQuery}
      oninput={handleInput}
      onkeydown={handleKeydown}
      class="w-full rounded-lg border border-slate-700 bg-slate-800 py-2 pl-10 pr-10 text-sm text-slate-100 placeholder-slate-500 focus:border-slate-500 focus:outline-none"
    />

    <!-- Clear button -->
    {#if agentsStore.searchQuery}
      <button
        onclick={() => {
          agentsStore.clearSearch();
          const input = document.querySelector('input[type="text"]') as HTMLInputElement;
          if (input) input.value = "";
        }}
        aria-label="Clear search"
        class="absolute right-3 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-200"
      >
        <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
          <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M6 18L18 6M6 6l12 12" />
        </svg>
      </button>
    {/if}
  </div>

  <!-- Scope dropdown -->
  <div class="relative" bind:this={dropdownRef}>
    <button
      onclick={() => (dropdownOpen = !dropdownOpen)}
      class="flex items-center gap-1 rounded-lg border border-slate-700 bg-slate-800 px-3 py-2 text-sm text-slate-300 hover:border-slate-500 hover:text-slate-100"
      title="Search scope"
    >
      <svg class="h-4 w-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path
          stroke-linecap="round"
          stroke-linejoin="round"
          stroke-width="2"
          d="M3 4a1 1 0 011-1h16a1 1 0 011 1v2.586a1 1 0 01-.293.707l-6.414 6.414a1 1 0 00-.293.707V17l-4 4v-6.586a1 1 0 00-.293-.707L3.293 7.293A1 1 0 013 6.586V4z"
        />
      </svg>
      {#if activeCount < 4}
        <span class="text-xs text-blue-400">{activeCount}</span>
      {/if}
    </button>

    {#if dropdownOpen}
      <div class="absolute right-0 top-full z-10 mt-1 w-48 rounded-lg border border-slate-700 bg-slate-800 py-1 shadow-lg">
        {#each Object.entries(scopeLabels) as [scope, label]}
          {@const checked = agentsStore.searchScopes.includes(scope as SearchScope)}
          {@const isLast = checked && agentsStore.searchScopes.length === 1}
          <button
            onclick={() => toggleScope(scope as SearchScope)}
            class="flex w-full items-center gap-2 px-3 py-1.5 text-sm hover:bg-slate-700 {isLast
              ? 'cursor-not-allowed opacity-50'
              : 'text-slate-300'}"
            disabled={isLast}
          >
            <span
              class="flex h-4 w-4 items-center justify-center rounded border {checked
                ? 'border-blue-500 bg-blue-500'
                : 'border-slate-500'}"
            >
              {#if checked}
                <svg class="h-3 w-3 text-white" fill="none" stroke="currentColor" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="3" d="M5 13l4 4L19 7" />
                </svg>
              {/if}
            </span>
            {label}
          </button>
        {/each}
      </div>
    {/if}
  </div>
</div>
