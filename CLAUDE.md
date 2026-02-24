# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
bun run dev              # Start both backend + frontend
bun run dev:backend      # Backend only (Rust/axum, port 3001)
bun run dev:frontend     # Frontend only (Vite, port 5173)
bun run build            # Build all packages
bun run test:backend     # Run Rust backend tests
bun run gen:types        # Generate TS types from Rust schemas
```

## Architecture

Monorepo with three packages:

- **`packages/backend`** — Rust (axum + tokio) HTTP + WebSocket server. Watches JSONL log files, runs state machine, broadcasts events. Types defined with serde + schemars.
- **`packages/shared`** — TypeScript types (generated from Rust JSON Schemas) and cost calculator. Imported by frontend via `@agents-dashboard/shared`.
- **`packages/frontend`** — SvelteKit 5 SPA (adapter-static, ssr=false) + Tailwind CSS 4. Connects to backend WebSocket for real-time updates.

### Rust Backend Structure

```
packages/backend/src/
├── main.rs                      # Entry point (axum server)
├── types.rs                     # Shared types (serde + schemars)
├── cost.rs                      # Token cost calculation
├── gen_schema.rs                # JSON Schema generation binary
├── providers/
│   ├── mod.rs                   # ProviderEvent enum
│   └── claude_code/
│       ├── mod.rs               # ClaudeCodeProvider
│       ├── state_machine.rs     # State machine
│       ├── session_discovery.rs # ~/.claude/projects/ scanner
│       ├── file_watcher.rs      # notify + polling
│       ├── jsonl_parser.rs      # JSONL parsing (RawEntry types)
│       └── message_mapper.rs    # RawEntry → AgentMessage
├── server/
│   ├── http.rs                  # axum Router (API + SPA fallback)
│   └── ws.rs                    # WebSocket handler (broadcast channels)
└── session/
    └── manager.rs               # SessionManager
```

### Type Sharing Pipeline

Rust types (serde + schemars) → JSON Schema → `json-schema-to-typescript` → `packages/shared/src/types/generated.d.ts`

### Data Flow

```
~/.claude/projects/<encoded-path>/<session-id>.jsonl
  → SessionDiscovery (scans for new .jsonl files every 5s)
  → FileWatcher (notify + 2s polling, incremental read with offset tracking)
  → JSONL parser → State machine + Message mapper
  → ClaudeCodeProvider (sends ProviderEvent via mpsc channel)
  → SessionManager → broadcast::channel<ServerEvent>
  → WebSocket handler (per-client tokio task) → Frontend
```

### State Machine (`state_machine.rs`)

States: `Running` | `Idle` | `PermissionWaiting` | `Error` | `Stopped`

- **→ Running**: any user/assistant/progress entry
- **→ Idle**: `system:turn_duration` entry detected
- **→ PermissionWaiting**: last entry was `assistant(tool_use)` + 10s silence
- **→ Stopped**: 60s no activity while Running (Idle stays Idle)
- **→ Error**: tool_result with `is_error: true`

### WebSocket Protocol (`/ws`)

Server→Client: `sessions:init`, `session:discovered`, `session:removed`, `session:state_changed`, `session:new_message`, `session:messages_init`, `session:usage_updated`

Client→Server: `subscribe:session`, `unsubscribe:session` (messages are only sent for subscribed sessions)

### WebSocket Architecture

- **broadcast channel**: all clients (discovered, removed, state_changed, usage_updated)
- **message channel**: subscription-filtered per client (new_message)
- Each WS connection runs as independent tokio task with send/receive loops

## Key Conventions

- Backend uses axum with tokio for async HTTP + WebSocket on port 3001
- Shared state via `Arc<RwLock<HashMap<...>>>` for thread-safe session storage
- `#[serde(rename_all = "camelCase")]` ensures wire-format compatibility with frontend
- `#[serde(tag = "type")]` for tagged union ServerEvent/ClientEvent serialization
- Frontend auto-connects WebSocket at module load (not in `onMount` — Svelte 5/SvelteKit hydration issue)
- Svelte 5 runes: stores use `$state` and `$derived` in `.svelte.ts` files
- Backend keeps last 500 messages per session; frontend keeps last 200
- Token costs calculated per-model using pricing table in both `cost.rs` and `cost-calculator.ts`
- Session metadata (project name, working directory) extracted from JSONL `cwd` field, not from directory encoding
