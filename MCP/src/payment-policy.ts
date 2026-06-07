import type { McpConfig } from "./types.js";

export interface PaymentRequirementLike {
  network?: string;
  asset?: string;
  assetId?: string;
  maxAmountRequired?: string;
  amount?: string;
  payTo?: string;
}

export function readConfig(): McpConfig {
  return {
    apiBase: process.env.NESTOR_API_BASE || "http://127.0.0.1:8080",
    paymentMode: parsePaymentMode(process.env.NESTOR_MCP_PAYMENT_MODE),
    devPaymentToken: process.env.NESTOR_MCP_DEV_PAYMENT_TOKEN || "dev-paid",
    expectedNetwork:
      process.env.NESTOR_MCP_EXPECTED_NETWORK ||
      "algorand:SGO1GKSzyE7IEPItTxCByw9x8FmnrCDexi9/cOUJOiI=",
    expectedAsset: process.env.NESTOR_MCP_EXPECTED_ASSET || "10458941",
    expectedPayTo: process.env.NESTOR_MCP_EXPECTED_PAY_TO || "",
    maxPriceAtomic: process.env.NESTOR_MCP_MAX_PRICE_ATOMIC || "1000",
    sessionMnemonic: process.env.NESTOR_MCP_SESSION_MNEMONIC || undefined,
  };
}

export function assertPaymentAllowed(
  requirement: PaymentRequirementLike,
  config: McpConfig,
  maxPriceAtomic?: string,
) {
  const requiredAmount = BigInt(requirement.maxAmountRequired || requirement.amount || "0");
  const allowedAmount = BigInt(maxPriceAtomic || config.maxPriceAtomic);
  const asset = requirement.assetId || requirement.asset;

  if (requirement.network && requirement.network !== config.expectedNetwork) {
    throw new Error(`Refusing payment on unexpected network: ${requirement.network}`);
  }

  if (asset && asset !== config.expectedAsset) {
    throw new Error(`Refusing payment for unexpected asset: ${asset}`);
  }

  if (requirement.payTo && config.expectedPayTo && requirement.payTo !== config.expectedPayTo) {
    throw new Error(`Refusing payment to unexpected receiver: ${requirement.payTo}`);
  }

  if (requiredAmount > allowedAmount) {
    throw new Error(`Refusing payment ${requiredAmount}; max allowed is ${allowedAmount}`);
  }
}

function parsePaymentMode(value?: string): McpConfig["paymentMode"] {
  if (value === "session" || value === "wallet" || value === "dev") return value;
  return "dev";
}
