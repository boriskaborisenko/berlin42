# Berlin42 Answer Forge

Hackathon project for Berlin42: a paid multi-model answer pipeline. A user pays for a request through x402 on Algorand testnet, sends a question like "какой самый умный кот?", and receives two useful artifacts:

- a polished final answer, assembled from several LLM perspectives
- a reusable prompt the user can paste into any LLM and adapt however they want

The current codebase started as TriForge, a skill/prompt/answer generator. The new product direction is narrower and sharper: paid premium answers plus portable prompts.

## What Works Now

- Rust backend with JSON API.
- React/Vite frontend for entering a request and watching pipeline progress.
- Vertex AI/Gemini integration through local service account credentials.
- In-memory background runs:
  - `POST /api/runs` starts a run and returns a `run_id`.
  - `GET /api/runs/:run_id` returns the latest snapshot for frontend polling.
- Live mode currently runs:
  - `base_llm` to turn the user request into a shared brief
  - `model_a`, `model_b`, and `model_c` in parallel for candidate generation
  - final formatter model call to produce the requested artifact
- Mock mode can run without model calls.

## Hackathon Direction

The intended Berlin42 flow:

1. User writes a question or task.
2. Frontend creates an x402 payment requirement.
3. User pays on Algorand testnet.
4. Backend verifies the payment proof.
5. Backend starts the multi-model pipeline.
6. Pipeline builds a strong answer from several model outputs.
7. Pipeline also returns a reusable prompt based on the final reasoning structure.
8. User sees the answer, prompt, and transparent pipeline progress.

x402/Algorand payment verification is the main missing product layer. The existing `/api/runs` endpoint is the right place to protect with payment verification.

## Pipeline

The master prompt chain lives in [prompts/master](prompts/master):

1. `00_base_brief.md`: converts the raw request into a shared task brief.
2. `01_candidate_generation.md`: asks three models to produce independent candidates.
3. `02_cross_review.md`: planned cross-review between model outputs.
4. `03_distributed_red_team.md`: planned stress test of weak assumptions.
5. `04_consensus_merge.md`: planned deterministic merge material.
6. `05_compression.md`: planned extraction of executable rules.
7. `06_eval.md`: planned checks against test cases.
8. `07_revision.md`: planned repair pass.
9. `08_final_formatter.md`: turns the pipeline material into the final answer or prompt.

Current live implementation uses real base brief, real parallel generation, and real final formatting. Cross-review, red-team, merge, compression, eval, and revision are still partial placeholders in the backend.

## API

When `X402_ENABLED=true`, live `POST /api/runs` is protected by x402. An unpaid request returns `402 Payment Required` with a Base64 JSON `PAYMENT-REQUIRED` header. The client retries with `PAYMENT-SIGNATURE`; the backend verifies and settles through the configured facilitator, then returns `PAYMENT-RESPONSE`.

Start a run:

```sh
curl -sS http://127.0.0.1:8080/api/runs \
  -H 'Content-Type: application/json' \
  -d '{"intent":"какой самый умный кот?","artifact_type":"answer","variant_count":1,"research_enabled":false,"execution_mode":"live"}'
```

Read a run snapshot:

```sh
curl -sS http://127.0.0.1:8080/api/runs/<run_id>
```

Test a configured Vertex model:

```sh
curl -sS http://127.0.0.1:8080/api/test-model \
  -H 'Content-Type: application/json' \
  -d '{"model_id":"base_llm","prompt":"Ответь одним предложением: какой самый умный кот?"}'
```

Other useful endpoints:

- `GET /health`
- `GET /api/schema`
- `GET /api/payment/requirements`
- `GET /api/models/config`

## Configuration

Secrets and runtime values come from environment variables. Non-secret model routing lives in [config_models.json](config_models.json).

Local Vertex credentials are resolved in this order:

1. `GOOGLE_APPLICATION_CREDENTIALS_JSON`
2. `GOOGLE_APPLICATION_CREDENTIALS`
3. `../llm.json` when running from `backend`

Never commit real credentials. `llm.json`, `.env`, and `IDEA.md` are intentionally ignored by Git and Docker.

x402 defaults target Algorand Testnet:

- facilitator: `https://facilitator.goplausible.xyz`
- network: `algorand:SGO1GKSzyE7IEPItTxCByw9x8FmnrCDexi9/cOUJOiI=`
- USDC ASA: `10458941`
- default amount: `1000` atomic units, or `0.001 USDC`
- receiving address: set `X402_PAY_TO`

## Development

Backend:

```sh
cd backend
cargo run
```

The backend loads the root `.env` automatically when started from `backend`.

x402 dev smoke test:

```sh
./scripts/smoke_x402_dev.sh
```

Expected shape:

- health returns `{"status":"ok"}`
- unpaid live run returns `402 Payment Required`
- dev-paid run starts when `X402_DEV_BYPASS=true`

Real x402 Algorand Testnet smoke test:

```sh
cd frontend
npm run x402:pay -- "какой самый умный кот?"
```

This uses `AVM_MNEMONIC` from the ignored root `.env`. The payer account must have TestNet ALGO, must be opted in to USDC ASA `10458941`, and must hold TestNet USDC.

Frontend:

```sh
cd frontend
npm install
npm run dev
```

Default local URLs:

- backend: `http://127.0.0.1:8080`
- frontend: Vite prints the dev URL, usually `http://localhost:5173`

## Docker

Build the backend image:

```sh
docker build -t berlin42-answer-forge .
```

Run it:

```sh
docker run --rm -p 8080:8080 \
  -e GOOGLE_CLOUD_PROJECT=<project-id> \
  -e GOOGLE_APPLICATION_CREDENTIALS_JSON='<service-account-json>' \
  berlin42-answer-forge
```

The Docker image includes `config_models.json` and `prompts/master`. Secrets must be injected at runtime.

## Project Map

- [backend](backend): Rust API server and pipeline orchestration
- [frontend](frontend): React/Vite interface
- [prompts/master](prompts/master): multi-model prompt chain
- [docs/architecture.md](docs/architecture.md): older architecture notes, still useful for context
- [docs/pipeline.md](docs/pipeline.md): detailed pipeline stage contract
- [infra/render.md](infra/render.md): Render deployment notes
- [config_models.json](config_models.json): non-secret model routing

## Next Steps

- Add x402 payment request and verification for Algorand testnet.
- Change the final run response to return both `answer` and `prompt` together.
- Wire live cross-review, red-team, merge, compression, eval, and revision stages.
- Persist runs instead of keeping them only in memory.
- Add frontend states for paid, verifying, running, completed, and failed payment.
