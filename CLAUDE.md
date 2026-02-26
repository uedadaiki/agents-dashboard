# CLAUDE.md

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

Monorepo: `packages/backend` (Rust/axum), `packages/shared` (generated TS types + cost calculator), `packages/frontend` (SvelteKit 5 SPA + Tailwind CSS 4).

Backend watches `~/.claude/projects/` JSONL log files, runs a state machine, and broadcasts events to the frontend via WebSocket.

### Type generation

Rust types (serde + schemars) → JSON Schema → `json-schema-to-typescript` → `packages/shared/src/types/generated.d.ts`. IMPORTANT: After changing Rust types in `types.rs`, run `bun run gen:types` to regenerate.

## Gotchas

- **Frontend WebSocket**: Auto-connects at module load, NOT in `onMount`. This is intentional — Svelte 5/SvelteKit hydration causes missed events if connected in `onMount`.
- **Svelte 5 runes**: Stores use `$state` and `$derived` in `.svelte.ts` files. Do not use legacy Svelte store syntax.
- **Serde attributes**: All Rust types use `#[serde(rename_all = "camelCase")]` and `#[serde(tag = "type")]` for tagged unions. Follow this convention for new types.
- **Session metadata**: Extracted from the JSONL `cwd` field, not from directory path encoding.
- **Message limits**: Backend keeps last 500 messages per session; frontend keeps last 200.
- **Token costs**: Maintained in both `cost.rs` (backend) and `cost-calculator.ts` (shared) — keep them in sync when updating.
