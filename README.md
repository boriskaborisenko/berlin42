# NestorChat

Paid answers should be better than a one-shot chatbot response.

NestorChat is a hackathon product that gates an expensive multi-model answer pipeline with **x402 payments on Algorand Testnet**. A user pays for one request, asks a question, and receives:

- a polished final answer;
- a reusable prompt they can paste into any LLM;
- a live quality check showing how the forged answer compares with a single-model baseline.

The product thesis is simple: a premium answer should survive drafting, criticism, red-team pressure, evaluation, and revision before the user sees it.

## Why It Exists

Single-model answers are fast, but they often miss edge cases, overstate uncertain facts, or produce fluent but shallow responses. NestorChat treats the first draft as raw material, not the final product.

Instead of:

```text
User question -> one model -> answer
```

NestorChat runs:

```text
User question
-> task brief
-> 3 independent candidates
-> cross-review
-> distributed red-team
-> consensus merge
-> compression
-> eval
-> revision
-> final answer + reusable prompt
-> async quality metrics
```

The result is designed to be more complete, more skeptical, and more useful than asking one model once.

## Product Flow

1. The user enters a request, for example: `Which is the smartest cat?`
2. The backend requires payment when `X402_ENABLED=true`.
3. After a valid x402 payment, the live LLM pipeline starts.
4. The frontend polls run progress and shows the final answer as markdown.
5. The user also gets a reusable prompt that captures the answer strategy.
6. A benchmark pass runs in the background and updates the quality card.

The web UI supports three payment modes:

- `Wallet`: connect Defly or Pera on Algorand TestNet and approve the x402 payment in the wallet.
- `Session`: use a small browser-local TestNet session wallet for fast demo auto-payments.
- `Dev`: send the local bypass header while developing.

For local development, `X402_DEV_BYPASS=true` allows the frontend to send `X402-DEV-PAYMENT: dev-paid` instead of making a real payment.

## What Makes It Valuable

**Better answers, not just longer answers.**  
The pipeline uses independent drafts, adversarial review, and revision gates to reduce shallow or overconfident output.

**Reusable prompt as a second artifact.**  
The user does not only get an answer. They get a portable prompt that lets them reproduce or adapt the same quality pattern elsewhere.

**Payment-gated AI compute.**  
x402 turns a high-cost multi-model pipeline into a paid API primitive. No payment, no expensive run.

**Per-answer quality evidence.**  
The UI shows a real-time comparison against a single `Gemini 3.5 Flash only` baseline, so the value of the consensus pipeline is visible in the demo.

## Quality Metrics

After the answer is ready, the backend starts an async benchmark pass using `benchmark_model`.

### Fact Confidence

Measures how many key claims in the final answer were supported by at least two model drafts.

```text
supported_claims / total_claims * 100
```

### Completeness Lift

Compares the final consensus answer against the best single raw draft.

```text
final_coverage - best_single_model_coverage
```

The frontend presents this as a head-to-head comparison:

```text
Gemini 3.5 Flash only     75%
NestorChat consensus   100%
```

## Current Status

Implemented:

- Rust backend with in-memory async run orchestration.
- React/Vite frontend with local history, markdown rendering, answer + prompt layout, and quality card.
- Vertex AI/Gemini integration via service account credentials.
- Live multi-stage pipeline: task brief, candidates, cross-review, red-team, merge, compression, eval, revision, final formatter.
- `reasoningEffort` mapping to Vertex `thinkingConfig` for supported Gemini models.
- x402 payment gate with Algorand Testnet config, Defly/Pera wallet mode, browser session wallet mode, and local dev bypass.
- Async per-run benchmark metrics.

Next focus:

- test the real Defly/Pera signing flow end to end;
- harden payment reuse/receipt handling for the demo;
- build the local MCP payment adapter;
- add offline benchmark reports;
- polish the UI around payment and run progress.

## Tech Stack

- **Backend:** Rust, raw HTTP server, background threads, in-memory run store.
- **Frontend:** React, Vite, CSS, `react-markdown`, `lucide-react`.
- **Models:** provider-routed through `config_models.json`; current live adapter uses Vertex AI / Gemini.
- **Payments:** x402 over Algorand Testnet.
- **Quality layer:** async benchmark model with FCI and coverage metrics.

## Model Providers

NestorChat is not meant to be locked to one model vendor. The pipeline roles are configured through `config_models.json`, so the same stages can be routed to different providers.

Current implementation:

- `vertex`: live Vertex AI / Gemini calls.
- `mock`: local mock stages for development.

Planned provider adapters:

- `openai`: GPT models for candidate generation, review, or final formatting.
- `anthropic`: Claude models for critique-heavy review, red-team, or synthesis.

The product value is the orchestration layer: independent drafts, criticism, red-team pressure, eval, revision, and a reusable prompt. Gemini, GPT, and Claude can all act as interchangeable workers inside that process once their adapters are wired.

## Key Files

```text
backend/              Rust API and pipeline orchestration
frontend/             React/Vite app
prompts/master/       Prompt chain for each pipeline stage
docs/                 Architecture, pipeline, benchmark, roadmap notes
scripts/              Smoke tests and helper scripts
config_models.json    Non-secret model routing
.env.example          Local environment template
Dockerfile            Backend container build
```

## Configuration

Secrets live in `.env` and must not be committed. Model routing lives in `config_models.json`.

Vertex credentials are resolved in this order:

1. `GOOGLE_APPLICATION_CREDENTIALS_JSON`
2. `GOOGLE_APPLICATION_CREDENTIALS`
3. `../llm.json` when running from `backend/`

Important x402 variables:

```env
X402_ENABLED=true
X402_FACILITATOR_URL=https://facilitator.goplausible.xyz
X402_NETWORK=algorand:SGO1GKSzyE7IEPItTxCByw9x8FmnrCDexi9/cOUJOiI=
X402_ASSET=10458941
X402_AMOUNT=1000
X402_PAY_TO=<algorand-testnet-receiver>
X402_DEV_BYPASS=true
X402_DEV_BYPASS_TOKEN=dev-paid
```

`10458941` is the Algorand Testnet USDC ASA used by the current demo config. `1000` atomic units equals `0.001 USDC`.

## Local Development

Install frontend dependencies once:

```sh
cd frontend
npm install
```

Run the backend:

```sh
cd backend
cargo run
```

Run the frontend:

```sh
cd frontend
npm run dev
```

Default URLs:

- backend: `http://127.0.0.1:8080`
- frontend: Vite will print the local URL, usually `http://localhost:5173`

## Useful API Calls

Start a live run with dev payment bypass:

```sh
curl -sS http://127.0.0.1:8080/api/runs \
  -H 'Content-Type: application/json' \
  -H 'X402-DEV-PAYMENT: dev-paid' \
  -d '{"intent":"Which is the smartest cat?","artifact_type":"answer","variant_count":1,"research_enabled":false,"execution_mode":"live"}'
```

Read a run snapshot:

```sh
curl -sS http://127.0.0.1:8080/api/runs/<run_id>
```

Test one model:

```sh
curl -sS http://127.0.0.1:8080/api/test-model \
  -H 'Content-Type: application/json' \
  -d '{"model_id":"base_llm","prompt":"Explain why the sky is blue in one sentence."}'
```

Payment requirement:

```sh
curl -sS http://127.0.0.1:8080/api/payment/requirements
```

## x402 CLI Smoke Tests

Dev bypass smoke test:

```sh
./scripts/smoke_x402_dev.sh
```

Real Algorand Testnet payment test:

```sh
cd frontend
npm run x402:pay -- "Which is the smartest cat?"
```

The payer account must have TestNet ALGO, be opted into USDC ASA `10458941`, and hold TestNet USDC.

## Docker

Build:

```sh
docker build -t nestorchat .
```

Run:

```sh
docker run --rm -p 8080:8080 \
  -e GOOGLE_CLOUD_PROJECT=<project-id> \
  -e GOOGLE_APPLICATION_CREDENTIALS_JSON='<service-account-json>' \
  -e X402_PAY_TO=<algorand-testnet-receiver> \
  nestorchat
```

Secrets should be injected at runtime. 
---

## AI Agent Payments via MCP

NestorChat can also be exposed to AI agents through the **Model Context Protocol (MCP)**.

### Why MCP?
AI agents cannot reliably click "Confirm" in a browser wallet for every paid tool call. A local MCP server can act as a payment adapter: it calls the NestorChat API, handles the x402 challenge, applies spend limits, and returns the answer plus reusable prompt to the agent.

### The Flow

```text
User Agent -> local NestorChat MCP server -> x402 payment adapter -> NestorChat API
```

1. The User Agent makes a tool call to the local MCP Server.
2. The MCP server makes the API request and receives `402 Payment Required`.
3. The payment adapter checks network, asset, recipient, timeout, and max price.
4. In autonomous mode, it signs with a small pre-funded session wallet.
5. In human-in-the-loop mode, it can request wallet approval instead.
6. The request is retried with `PAYMENT-SIGNATURE`, settled on Algorand TestNet, and the answer is returned to the agent.

Seed phrases are acceptable for bounded TestNet session wallets, but they should not be the main human wallet UX. Humans should use Defly/Pera; agents should use limited session wallets with strict spend caps.

For a detailed architectural breakdown of this flow, see [MCP/concept.md](MCP/concept.md).
