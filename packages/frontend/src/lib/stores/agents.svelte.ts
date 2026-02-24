import type { AgentMessage, AgentSessionSummary, AgentStateType, CumulativeUsage, SearchResponse, SearchScope, SessionSearchResult, ServerEvent } from "@agents-dashboard/shared";

const ALL_SCOPES: SearchScope[] = ["project_name", "current_task", "working_directory", "content"];

export type RecencyFilter = "1h" | "6h" | "24h" | "7d" | "30d" | "all";

const RECENCY_STORAGE_KEY = "agents-dashboard-recency-filter";

function loadRecencyFilter(): RecencyFilter {
  try {
    const stored = localStorage.getItem(RECENCY_STORAGE_KEY);
    if (stored && ["1h", "6h", "24h", "7d", "30d", "all"].includes(stored)) {
      return stored as RecencyFilter;
    }
  } catch {}
  return "all";
}

class AgentsStore {
  sessions = $state<AgentSessionSummary[]>([]);
  // Messages per session (for detail view)
  sessionMessages = $state<Record<string, AgentMessage[]>>({});

  // Recency filter
  recencyFilter = $state<RecencyFilter>(loadRecencyFilter());

  // Search state
  searchQuery = $state("");
  searchScopes = $state<SearchScope[]>([...ALL_SCOPES]);
  searchResults = $state<SearchResponse | null>(null);
  isSearching = $state(false);

  setRecencyFilter(value: RecencyFilter): void {
    this.recencyFilter = value;
    try {
      localStorage.setItem(RECENCY_STORAGE_KEY, value);
    } catch {}
  }

  handleEvent(event: ServerEvent): void {
    switch (event.type) {
      case "sessions:init":
        this.sessions = event.sessions;
        break;

      case "session:discovered":
        this.sessions = [...this.sessions, event.session];
        break;

      case "session:removed":
        this.sessions = this.sessions.filter((s) => s.sessionId !== event.sessionId);
        delete this.sessionMessages[event.sessionId];
        break;

      case "session:state_changed":
        this.sessions = this.sessions.map((s) =>
          s.sessionId === event.sessionId ? { ...event.session } : s,
        );
        break;

      case "session:messages_init": {
        // Replace existing messages with the full backlog from the server
        this.sessionMessages[event.sessionId] = event.messages.slice(-200);
        break;
      }

      case "session:new_message": {
        const msgs = this.sessionMessages[event.sessionId] ?? [];
        const updated = [...msgs, event.message];
        this.sessionMessages[event.sessionId] = updated.length > 200 ? updated.slice(-150) : updated;
        break;
      }

      case "session:usage_updated":
        this.sessions = this.sessions.map((s) =>
          s.sessionId === event.sessionId ? { ...s, cumulativeUsage: event.usage } : s,
        );
        break;

      case "session:git_status_updated":
        this.sessions = this.sessions.map((s) =>
          s.sessionId === event.sessionId ? { ...s, gitStatus: event.gitStatus } : s,
        );
        break;
    }
  }

  getSession(sessionId: string): AgentSessionSummary | undefined {
    return this.sessions.find((s) => s.sessionId === sessionId);
  }

  getMessages(sessionId: string): AgentMessage[] {
    return this.sessionMessages[sessionId] ?? [];
  }

  async fetchMessages(sessionId: string): Promise<void> {
    try {
      const res = await fetch(`http://${window.location.hostname}:3001/api/sessions/${sessionId}`);
      if (!res.ok) return;
      const detail = await res.json();
      if (detail.messages?.length) {
        // Only set if we don't already have messages (avoid overwriting live updates)
        if (!this.sessionMessages[sessionId]?.length) {
          this.sessionMessages[sessionId] = detail.messages.slice(-200);
        }
      }
    } catch {
      // Ignore fetch errors
    }
  }

  async search(query: string): Promise<void> {
    this.searchQuery = query;
    if (!query.trim()) {
      this.searchResults = null;
      this.isSearching = false;
      return;
    }
    this.isSearching = true;
    try {
      const scopeParam = this.searchScopes.join(",");
      const res = await fetch(
        `http://${window.location.hostname}:3001/api/search?q=${encodeURIComponent(query)}&scope=${scopeParam}`,
      );
      if (!res.ok) return;
      const data: SearchResponse = await res.json();
      // Only update if query hasn't changed during the fetch
      if (this.searchQuery === query) {
        this.searchResults = data;
      }
    } catch {
      // Ignore fetch errors
    } finally {
      if (this.searchQuery === query) {
        this.isSearching = false;
      }
    }
  }

  clearSearch(): void {
    this.searchQuery = "";
    this.searchResults = null;
    this.isSearching = false;
  }

  getSearchResult(sessionId: string): SessionSearchResult | undefined {
    return this.searchResults?.results.find((r) => r.session.sessionId === sessionId);
  }

  toggleScope(scope: SearchScope): void {
    const idx = this.searchScopes.indexOf(scope);
    if (idx >= 0) {
      // Don't remove the last scope
      if (this.searchScopes.length <= 1) return;
      this.searchScopes = this.searchScopes.filter((s) => s !== scope);
    } else {
      this.searchScopes = [...this.searchScopes, scope];
    }
    // Re-search if there's an active query
    if (this.searchQuery.trim()) {
      this.search(this.searchQuery);
    }
  }
}

export const agentsStore = new AgentsStore();
