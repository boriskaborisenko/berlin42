export type PaymentMode = "dev" | "session" | "wallet";

export type ToolMode = "live" | "mock";

export interface PremiumAnswerInput {
  question: string;
  mode?: ToolMode;
  max_price_atomic?: string;
  include_quality_metrics?: boolean;
}

export interface PremiumAnswerOutput {
  question: string;
  answer: string;
  reusable_prompt: string;
  run_id?: string;
  status?: string;
  quality_metrics?: unknown;
  payment?: {
    mode: PaymentMode;
    network?: string;
    asset?: string;
    amount_atomic?: string;
    receipt?: unknown;
  };
}

export interface NestorRun {
  run_id: string;
  status: "queued" | "running" | "completed" | "error";
  intent?: string;
  error?: string;
  benchmark_status?: string;
  quality_metrics?: unknown;
  final_variants?: Array<{
    content?: string;
    reusable_prompt?: string;
  }>;
}

export interface McpConfig {
  apiBase: string;
  paymentMode: PaymentMode;
  devPaymentToken: string;
  expectedNetwork: string;
  expectedAsset: string;
  expectedPayTo: string;
  maxPriceAtomic: string;
  sessionMnemonic?: string;
}
