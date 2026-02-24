import type { CumulativeUsage } from "../types/index.js";

// Pricing per million tokens (USD) — Claude model pricing
// Uses prefix matching to handle versioned model names (e.g. claude-opus-4-1-20250805)
const MODEL_PRICING: Array<{ prefix: string; pricing: { input: number; output: number; cacheRead: number; cacheCreation: number } }> = [
  { prefix: "claude-opus", pricing: { input: 15, output: 75, cacheRead: 1.5, cacheCreation: 18.75 } },
  { prefix: "claude-sonnet", pricing: { input: 3, output: 15, cacheRead: 0.3, cacheCreation: 3.75 } },
  { prefix: "claude-haiku", pricing: { input: 0.8, output: 4, cacheRead: 0.08, cacheCreation: 1 } },
];

const DEFAULT_PRICING = MODEL_PRICING[1].pricing; // sonnet

function getPricing(model: string) {
  const match = MODEL_PRICING.find((m) => model.startsWith(m.prefix));
  return match?.pricing ?? DEFAULT_PRICING;
}

export function calculateCost(
  model: string,
  inputTokens: number,
  outputTokens: number,
  cacheReadTokens: number,
  cacheCreationTokens: number,
): number {
  const pricing = getPricing(model);
  return (
    (inputTokens * pricing.input +
      outputTokens * pricing.output +
      cacheReadTokens * pricing.cacheRead +
      cacheCreationTokens * pricing.cacheCreation) /
    1_000_000
  );
}

export function createEmptyUsage(): CumulativeUsage {
  return {
    inputTokens: 0,
    outputTokens: 0,
    cacheReadTokens: 0,
    cacheCreationTokens: 0,
    estimatedCost: 0,
  };
}

export function addUsage(
  current: CumulativeUsage,
  model: string,
  inputTokens: number,
  outputTokens: number,
  cacheReadTokens: number,
  cacheCreationTokens: number,
): CumulativeUsage {
  const entryCost = calculateCost(model, inputTokens, outputTokens, cacheReadTokens, cacheCreationTokens);
  return {
    inputTokens: current.inputTokens + inputTokens,
    outputTokens: current.outputTokens + outputTokens,
    cacheReadTokens: current.cacheReadTokens + cacheReadTokens,
    cacheCreationTokens: current.cacheCreationTokens + cacheCreationTokens,
    estimatedCost: current.estimatedCost + entryCost,
  };
}
