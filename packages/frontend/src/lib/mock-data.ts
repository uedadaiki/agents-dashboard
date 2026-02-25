import type { AgentSessionSummary, AgentMessage } from "@agents-dashboard/shared";

export function isMockMode(): boolean {
  if (typeof window === "undefined") return false;
  return new URLSearchParams(window.location.search).has("mock");
}

const now = Date.now();
const min = 60_000;

function ts(offsetMs: number): string {
  return new Date(now - offsetMs).toISOString();
}

let msgId = 0;
function msg(
  sessionId: string,
  overrides: Partial<AgentMessage> & Pick<AgentMessage, "role" | "type" | "content">,
  offsetMs: number,
): AgentMessage {
  return {
    id: `mock-${++msgId}`,
    sessionId,
    timestamp: ts(offsetMs),
    metadata: null,
    ...overrides,
  };
}

// ── Sessions ────────────────────────────────────────────────────────

export const mockSessions: AgentSessionSummary[] = [
  {
    sessionId: "a1b2c3d4-1111-4000-8000-000000000001",
    provider: "claude-code",
    projectName: "web-app",
    projectPath: "/Users/dev/projects/web-app",
    workingDirectory: "/Users/dev/projects/web-app",
    model: "claude-sonnet-4-20250514",
    state: "running",
    currentTask: "Add user authentication with JWT tokens and role-based access control",
    startedAt: ts(32 * min),
    lastActivityAt: ts(10_000),
    cumulativeUsage: {
      inputTokens: 184_320,
      outputTokens: 12_800,
      cacheReadTokens: 92_000,
      cacheCreationTokens: 45_000,
      estimatedCost: 0.42,
    },
    gitStatus: { branch: "feat/auth", additions: 247, deletions: 38 },
  },
  {
    sessionId: "a1b2c3d4-2222-4000-8000-000000000002",
    provider: "claude-code",
    projectName: "api-server",
    projectPath: "/Users/dev/projects/api-server",
    workingDirectory: "/Users/dev/projects/api-server",
    model: "claude-sonnet-4-20250514",
    state: "idle",
    currentTask: "Refactor database connection pooling for better performance",
    startedAt: ts(85 * min),
    lastActivityAt: ts(4 * min),
    cumulativeUsage: {
      inputTokens: 312_000,
      outputTokens: 28_400,
      cacheReadTokens: 156_000,
      cacheCreationTokens: 78_000,
      estimatedCost: 0.87,
    },
    gitStatus: { branch: "refactor/db-pool", additions: 189, deletions: 142 },
  },
  {
    sessionId: "a1b2c3d4-3333-4000-8000-000000000003",
    provider: "claude-code",
    projectName: "mobile-app",
    projectPath: "/Users/dev/projects/mobile-app",
    workingDirectory: "/Users/dev/projects/mobile-app",
    model: "claude-opus-4-20250514",
    state: "permission_waiting",
    currentTask: "Fix image upload handling and add compression",
    startedAt: ts(18 * min),
    lastActivityAt: ts(45_000),
    cumulativeUsage: {
      inputTokens: 95_000,
      outputTokens: 8_200,
      cacheReadTokens: 48_000,
      cacheCreationTokens: 24_000,
      estimatedCost: 0.68,
    },
    gitStatus: { branch: "fix/image-upload", additions: 63, deletions: 12 },
  },
  {
    sessionId: "a1b2c3d4-4444-4000-8000-000000000004",
    provider: "claude-code",
    projectName: "data-pipeline",
    projectPath: "/Users/dev/projects/data-pipeline",
    workingDirectory: "/Users/dev/projects/data-pipeline",
    model: "claude-sonnet-4-20250514",
    state: "running",
    currentTask: "Implement real-time data streaming with WebSocket support",
    startedAt: ts(12 * min),
    lastActivityAt: ts(5_000),
    cumulativeUsage: {
      inputTokens: 67_000,
      outputTokens: 5_100,
      cacheReadTokens: 33_000,
      cacheCreationTokens: 17_000,
      estimatedCost: 0.19,
    },
    gitStatus: { branch: "feat/streaming", additions: 312, deletions: 45 },
  },
  {
    sessionId: "a1b2c3d4-5555-4000-8000-000000000005",
    provider: "claude-code",
    projectName: "cli-tool",
    projectPath: "/Users/dev/projects/cli-tool",
    workingDirectory: "/Users/dev/projects/cli-tool",
    model: "claude-haiku-4-5-20251001",
    state: "stopped",
    currentTask: "Add config file parsing and validation",
    startedAt: ts(120 * min),
    lastActivityAt: ts(65 * min),
    cumulativeUsage: {
      inputTokens: 420_000,
      outputTokens: 38_000,
      cacheReadTokens: 210_000,
      cacheCreationTokens: 105_000,
      estimatedCost: 0.15,
    },
    gitStatus: { branch: "feat/config", additions: 534, deletions: 89 },
  },
  {
    sessionId: "a1b2c3d4-6666-4000-8000-000000000006",
    provider: "claude-code",
    projectName: "e-commerce",
    projectPath: "/Users/dev/projects/e-commerce",
    workingDirectory: "/Users/dev/projects/e-commerce",
    model: "claude-sonnet-4-20250514",
    state: "error",
    currentTask: "Debug checkout payment processing flow",
    startedAt: ts(45 * min),
    lastActivityAt: ts(8 * min),
    cumulativeUsage: {
      inputTokens: 245_000,
      outputTokens: 19_600,
      cacheReadTokens: 122_000,
      cacheCreationTokens: 61_000,
      estimatedCost: 0.58,
    },
    gitStatus: { branch: "fix/checkout", additions: 78, deletions: 23 },
  },
];

// ── Messages (for first session detail view) ────────────────────────

function buildMessages(sessionId: string): AgentMessage[] {
  return [
    msg(sessionId, {
      role: "user",
      type: "text",
      content: "Add JWT-based authentication to the Express server. Include signup, login, and a middleware that protects routes. Use bcrypt for password hashing.",
    }, 30 * min),

    msg(sessionId, {
      role: "assistant",
      type: "text",
      content: "I'll implement JWT authentication for the Express server. Let me start by examining the current project structure and dependencies.",
    }, 29.5 * min),

    msg(sessionId, {
      role: "assistant",
      type: "tool_use",
      content: "Read src/server.ts",
      metadata: { tool: "Read", path: "src/server.ts" },
    }, 29 * min),

    msg(sessionId, {
      role: "assistant",
      type: "tool_result",
      content: "import express from 'express';\nimport cors from 'cors';\n\nconst app = express();\napp.use(cors());\napp.use(express.json());\n\n// Routes\napp.get('/api/health', (req, res) => res.json({ status: 'ok' }));\n\napp.listen(3000, () => console.log('Server running on port 3000'));",
    }, 28.8 * min),

    msg(sessionId, {
      role: "assistant",
      type: "tool_use",
      content: "Bash npm install jsonwebtoken bcrypt && npm install -D @types/jsonwebtoken @types/bcrypt",
      metadata: { tool: "Bash" },
    }, 28 * min),

    msg(sessionId, {
      role: "assistant",
      type: "tool_result",
      content: "added 12 packages in 3.2s",
    }, 27.5 * min),

    msg(sessionId, {
      role: "assistant",
      type: "text",
      content: "Now I'll create the authentication module with the User model, auth routes, and middleware.",
    }, 27 * min),

    msg(sessionId, {
      role: "assistant",
      type: "tool_use",
      content: "Edit src/auth/middleware.ts — Create JWT verification middleware",
      metadata: { tool: "Edit", path: "src/auth/middleware.ts" },
    }, 26 * min),

    msg(sessionId, {
      role: "assistant",
      type: "tool_result",
      content: "Created src/auth/middleware.ts",
    }, 25.8 * min),

    msg(sessionId, {
      role: "assistant",
      type: "tool_use",
      content: "Edit src/auth/routes.ts — Create signup and login endpoints",
      metadata: { tool: "Edit", path: "src/auth/routes.ts" },
    }, 25 * min),

    msg(sessionId, {
      role: "assistant",
      type: "tool_result",
      content: "Created src/auth/routes.ts",
    }, 24.8 * min),

    msg(sessionId, {
      role: "assistant",
      type: "tool_use",
      content: "Edit src/server.ts — Wire up auth routes and middleware",
      metadata: { tool: "Edit", path: "src/server.ts" },
    }, 24 * min),

    msg(sessionId, {
      role: "assistant",
      type: "tool_result",
      content: "Modified src/server.ts",
    }, 23.8 * min),

    msg(sessionId, {
      role: "assistant",
      type: "tool_use",
      content: "Grep pattern=\"password\" glob=\"**/*.ts\"",
      metadata: { tool: "Grep" },
    }, 23 * min),

    msg(sessionId, {
      role: "assistant",
      type: "tool_result",
      content: "src/auth/routes.ts:12: const hashedPassword = await bcrypt.hash(password, 12);\nsrc/auth/routes.ts:28: const valid = await bcrypt.compare(password, user.password);",
    }, 22.8 * min),

    msg(sessionId, {
      role: "assistant",
      type: "tool_use",
      content: "Bash npm test",
      metadata: { tool: "Bash" },
    }, 22 * min),

    msg(sessionId, {
      role: "assistant",
      type: "tool_result",
      content: "FAIL src/auth/__tests__/routes.test.ts\n  ● POST /auth/signup › should return 400 if email is missing\n    Expected status 400, received 500",
      metadata: { is_error: true },
    }, 21.5 * min),

    msg(sessionId, {
      role: "assistant",
      type: "text",
      content: "The test revealed a missing validation check. Let me fix the signup route to properly validate required fields before processing.",
    }, 21 * min),

    msg(sessionId, {
      role: "assistant",
      type: "tool_use",
      content: "Edit src/auth/routes.ts — Add input validation for signup",
      metadata: { tool: "Edit", path: "src/auth/routes.ts" },
    }, 20 * min),

    msg(sessionId, {
      role: "assistant",
      type: "tool_result",
      content: "Modified src/auth/routes.ts",
    }, 19.8 * min),

    msg(sessionId, {
      role: "assistant",
      type: "tool_use",
      content: "Bash npm test",
      metadata: { tool: "Bash" },
    }, 19 * min),

    msg(sessionId, {
      role: "assistant",
      type: "tool_result",
      content: "PASS src/auth/__tests__/routes.test.ts\n  POST /auth/signup\n    ✓ should create user and return token (45ms)\n    ✓ should return 400 if email is missing (12ms)\n    ✓ should return 409 if email exists (18ms)\n  POST /auth/login\n    ✓ should return token for valid credentials (38ms)\n    ✓ should return 401 for invalid password (15ms)\n\nTest Suites: 1 passed, 1 total\nTests:       5 passed, 5 total",
    }, 18.5 * min),

    msg(sessionId, {
      role: "system",
      type: "state_change",
      content: "Turn completed (42.3s)",
    }, 18 * min),
  ];
}

// Build messages for the two "running" sessions and the permission_waiting one
const sessionsWithMessages = [
  mockSessions[0].sessionId,
  mockSessions[2].sessionId,
  mockSessions[3].sessionId,
];

export const mockSessionMessages: Record<string, AgentMessage[]> = {};
for (const sid of sessionsWithMessages) {
  mockSessionMessages[sid] = buildMessages(sid);
}
