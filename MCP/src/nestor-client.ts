import type { McpConfig, NestorRun, PremiumAnswerInput, PremiumAnswerOutput } from "./types.js";

const POLL_INTERVAL_MS = 1200;
const MAX_POLLS = 180;

export async function getPremiumAnswer(
  input: PremiumAnswerInput,
  config: McpConfig,
): Promise<PremiumAnswerOutput> {
  const response = await startRun(input, config);
  const run = response.status === "completed" ? response : await pollRun(response.run_id, config);
  const finalVariant = run.final_variants?.[0] || {};

  if (run.status === "error") {
    throw new Error(run.error || "NestorChat run failed.");
  }

  return {
    question: run.intent || input.question,
    answer: finalVariant.content || "",
    reusable_prompt: finalVariant.reusable_prompt || "",
    run_id: run.run_id,
    status: run.status,
    quality_metrics: input.include_quality_metrics === false ? undefined : run.quality_metrics,
    payment: {
      mode: config.paymentMode,
      network: config.expectedNetwork,
      asset: config.expectedAsset,
      amount_atomic: input.max_price_atomic || config.maxPriceAtomic,
    },
  };
}

async function startRun(input: PremiumAnswerInput, config: McpConfig): Promise<NestorRun> {
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
  };

  if (config.paymentMode === "dev") {
    headers["X402-DEV-PAYMENT"] = config.devPaymentToken;
  }

  if (config.paymentMode === "session") {
    // TODO: wrap fetch with @x402/fetch and @x402/avm ExactAvmScheme.
    // Before signing, parse the 402 requirement and call assertPaymentAllowed().
    throw new Error("session payment mode is scaffolded but not implemented yet.");
  }

  if (config.paymentMode === "wallet") {
    throw new Error("wallet payment mode needs a WalletConnect bridge and is not implemented yet.");
  }

  const response = await fetch(`${config.apiBase}/api/runs`, {
    method: "POST",
    headers,
    body: JSON.stringify({
      intent: input.question,
      artifact_type: "answer",
      variant_count: 1,
      research_enabled: false,
      execution_mode: input.mode || "live",
    }),
  });

  const data = (await response.json()) as NestorRun & { error?: string };
  if (!response.ok || data.error) {
    throw new Error(data.error || `NestorChat API failed with ${response.status}`);
  }

  return data;
}

async function pollRun(runId: string, config: McpConfig): Promise<NestorRun> {
  for (let attempt = 0; attempt < MAX_POLLS; attempt += 1) {
    await sleep(POLL_INTERVAL_MS);

    const response = await fetch(`${config.apiBase}/api/runs/${runId}`);
    const data = (await response.json()) as NestorRun & { error?: string };

    if (!response.ok || data.error) {
      throw new Error(data.error || `NestorChat polling failed with ${response.status}`);
    }

    if (data.status === "completed" || data.status === "error") {
      return data;
    }
  }

  throw new Error(`Timed out waiting for NestorChat run ${runId}`);
}

function sleep(ms: number) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
