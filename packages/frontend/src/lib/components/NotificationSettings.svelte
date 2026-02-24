<script lang="ts">
  import { notificationsStore } from "$lib/stores/notifications.svelte.js";

  let open = $state(false);
</script>

<div class="relative">
  <button
    onclick={() => (open = !open)}
    class="rounded-lg p-1.5 text-slate-400 hover:bg-slate-700 hover:text-slate-200"
    title="Notification settings"
  >
    <svg class="h-5 w-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
      <path
        stroke-linecap="round"
        stroke-linejoin="round"
        stroke-width="2"
        d="M15 17h5l-1.405-1.405A2.032 2.032 0 0118 14.158V11a6.002 6.002 0 00-4-5.659V5a2 2 0 10-4 0v.341C7.67 6.165 6 8.388 6 11v3.159c0 .538-.214 1.055-.595 1.436L4 17h5m6 0v1a3 3 0 11-6 0v-1m6 0H9"
      />
    </svg>
  </button>

  {#if open}
    <div class="absolute right-0 top-full z-50 mt-2 w-64 rounded-lg border border-slate-700 bg-slate-800 p-4 shadow-xl">
      <h3 class="mb-3 text-sm font-medium text-slate-200">Notifications</h3>

      <label class="flex items-center justify-between py-1">
        <span class="text-xs text-slate-300">Enabled</span>
        <input
          type="checkbox"
          checked={notificationsStore.settings.enabled}
          onchange={() => notificationsStore.toggle("enabled")}
          class="rounded"
        />
      </label>

      <label class="flex items-center justify-between py-1">
        <span class="text-xs text-slate-300">Desktop</span>
        <input
          type="checkbox"
          checked={notificationsStore.settings.desktopEnabled}
          onchange={() => {
            notificationsStore.requestPermission();
            notificationsStore.toggle("desktopEnabled");
          }}
          class="rounded"
        />
      </label>

      <label class="flex items-center justify-between py-1">
        <span class="text-xs text-slate-300">Sound</span>
        <input
          type="checkbox"
          checked={notificationsStore.settings.soundEnabled}
          onchange={() => notificationsStore.toggle("soundEnabled")}
          class="rounded"
        />
      </label>

      {#if notificationsStore.settings.desktopEnabled && notificationsStore.permissionState === "default"}
        <button
          onclick={() => notificationsStore.requestPermission()}
          class="mt-2 w-full rounded bg-blue-600 px-2 py-1.5 text-xs font-medium text-white hover:bg-blue-500"
        >
          Allow desktop notifications
        </button>
      {:else if notificationsStore.permissionState === "denied"}
        <p class="mt-2 text-xs text-red-400">
          Notifications blocked. Enable in browser/system settings.
        </p>
      {/if}

      <hr class="my-2 border-slate-700" />
      <p class="mb-1 text-xs text-slate-400">Notify on:</p>

      <label class="flex items-center justify-between py-1">
        <span class="text-xs text-slate-300">Idle</span>
        <input
          type="checkbox"
          checked={notificationsStore.settings.notifyOn.idle}
          onchange={() => notificationsStore.toggleNotifyOn("idle")}
          class="rounded"
        />
      </label>

      <label class="flex items-center justify-between py-1">
        <span class="text-xs text-slate-300">Permission waiting</span>
        <input
          type="checkbox"
          checked={notificationsStore.settings.notifyOn.permissionWaiting}
          onchange={() => notificationsStore.toggleNotifyOn("permissionWaiting")}
          class="rounded"
        />
      </label>

      <label class="flex items-center justify-between py-1">
        <span class="text-xs text-slate-300">Error</span>
        <input
          type="checkbox"
          checked={notificationsStore.settings.notifyOn.error}
          onchange={() => notificationsStore.toggleNotifyOn("error")}
          class="rounded"
        />
      </label>
    </div>
  {/if}
</div>
