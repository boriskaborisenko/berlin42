# Berlin42 Answer Forge

Hackathon project for Berlin42: a paid multi-model answer pipeline. A user pays for a request through x402 on Algorand testnet, sends a question (e.g., "Which is the smartest cat?"), and receives two useful, high-value artifacts:

- **Polished final answer**: assembled from several independent LLM perspectives
- **Reusable prompt**: a structured prompt based on the final reasoning structure that the user can paste into any LLM and adapt however they want

---

## Core Essence & Value Proposition

Direct single-model prompt responses often struggle with hallucinations, lack of nuance, or formatting issues. Answer Forge solves this by routing requests through a multi-model consensus pipeline. 

By paying a micro-amount via x402 on Algorand testnet, users gain access to a powerful pipeline that synthesizes opinions from multiple independent LLM drafts, reviews assumptions, resolves contradictions, and delivers a highly trustworthy answer paired with a professional portable prompt.

---

## Real-Time Quality Benchmarking

To prove why a paid, consensus-driven answer is stronger than a direct single-model output, Answer Forge runs a real-time, asynchronous benchmarking engine immediately after an answer is generated.

These metrics are calculated dynamically for each individual request against the raw drafts generated during the pipeline run:

### 1. Fact Confidence Index (FCI) — "Consensus Confidence"
FCI estimates how many key factual claims in the final answer were independently supported by at least two model candidates.
*   **Formula**: `FCI = (claims_supported_by_2_or_more_models / total_claims) * 100`
*   **UI Explanation**: Checks how many key claims in the final answer were supported by at least two independent model drafts.
*   *Example*: 94% consensus confidence. 15 of 16 key claims were supported by at least two model drafts.

### 2. Aspect Coverage Gap (ACG) — "Coverage Lift"
ACG measures whether the final consensus answer covers more important aspects of the question than any single candidate model draft alone.
*   **Formula**: 
    ```text
    final_coverage = covered_aspects_by_final / total_aspects * 100
    best_single_model_coverage = max(model_a, model_b, model_c)
    coverage_gain = final_coverage - best_single_model_coverage
    ```
*   **UI Explanation**: Compares how many important aspects the final answer covers versus the best single model draft.
*   *Example*: 100% coverage (+20% lift). The final answer covers 5 of 5 important aspects, +20% over the best single draft.

### Async Evaluation Flow
To avoid delaying the user experience, the pipeline returns the final answer immediately, and then spins up the `benchmark_model` (e.g., `gemini-3.5-flash` or similar) in a background thread to calculate these quality metrics. The frontend polls for updates and displays the metrics card dynamically once they are ready.

---

## Static Evaluation Track & Scoring Rubric

In addition to real-time user benchmarking, Answer Forge has an offline evaluation track designed to run automated regression checks against a standardized test set across four categories:
1.  **Everyday Practical**: e.g., "How to boil eggs?", "How to quickly clean an apartment before guests?"
2.  **Explanatory**: e.g., "Why is the sky blue? Explain simply.", "Explain blockchain to someone with no technical background"
3.  **Ambiguous**: e.g., "Which is the smartest cat?", "How to become more productive?"
4.  **High-Risk / Current-Facts**: e.g., "What tax scheme should a freelancer choose?", "What are the best laptops under $1000 right now?"

### Scoring Rubric (0-5 scale)
Each pipeline output is evaluated on six distinct dimensions:
1.  **Directness**: Does it answer the actual question quickly? (0 = no answer, 5 = clear answer first)
2.  **Usefulness**: Can the user act on it immediately? (0 = generic, 5 = highly concrete and complete)
3.  **Nuance**: Does it handle assumptions, edge cases, and tradeoffs? (0 = one-size-fits-all, 5 = handles crucial caveats elegantly)
4.  **Correctness & Safety**: Does it avoid wrong, unsafe, or overconfident claims? (0 = harmful/wrong, 5 = accurate, bounded, honest)
5.  **Structure**: Is it easy to read and scan? (0 = messy wall of text, 5 = polished markdown with strong hierarchy)
6.  **Reusable Prompt Quality**: Only scored for the Forge output. (0 = missing, 5 = strong standalone prompt with role, task, constraints, and self-checks)

### Win Condition
The Forge pipeline wins a test case if:
*   The overall Forge answer score is higher than the single-model baseline by at least **15%**, AND
*   Forge has a `Correctness & Safety` score that is equal to or higher than the baseline, AND
*   The Forge reusable prompt score is at least **3**.

---

## What Works Now

- **Rust Backend**: High-performance JSON API.
- **React/Vite Frontend**: Premium, centered UI in a beautiful purple/indigo theme.
- **Vertex AI/Gemini Integration**: Secured via service account credentials.
- **In-Memory Background Runs**:
  - `POST /api/runs` initiates a consensus run and returns a `run_id`.
  - `GET /api/runs/:run_id` provides live polling updates of the stage progress.
- **Core Pipeline Implementation**:
  - `base_llm` turns raw queries into a shared brief.
  - `model_a`, `model_b`, and `model_c` run in parallel to generate candidates.
  - Final formatter builds the polished consensus answer and prompt.
- **Mock Mode**: Fully operational for testing without model calls.

---

## Algorand/x402 Integration

When `X402_ENABLED=true` is set, live `POST /api/runs` calls require payment.
1. Unpaid requests return `402 Payment Required` with a Base64 JSON `PAYMENT-REQUIRED` header.
2. The client processes the payment and retries with the `PAYMENT-SIGNATURE` header.
3. The backend verifies the signature through the facilitator and returns `PAYMENT-RESPONSE`, unlocking the pipeline run.

---

## API Endpoints

### Start a Run
```sh
curl -sS http://127.0.0.1:8080/api/runs \
  -H 'Content-Type: application/json' \
  -d '{"intent":"Which is the smartest cat?","artifact_type":"answer","variant_count":1,"research_enabled":false,"execution_mode":"live"}'
```

### Read a Run Snapshot
```sh
curl -sS http://127.0.0.1:8080/api/runs/<run_id>
```

### Test a Specific Model Configuration
```sh
curl -sS http://127.0.0.1:8080/api/test-model \
  -H 'Content-Type: application/json' \
  -d '{"model_id":"base_llm","prompt":"Explain in one sentence: why is the sky blue?"}'
```

### Utility Endpoints
*   `GET /health`
*   `GET /api/schema`
*   `GET /api/payment/requirements`
*   `GET /api/models/config`

---

## Configuration

Secrets and runtime values are managed via environment variables. Non-secret model routing is located in `config_models.json`.

Local Vertex credentials are resolved in the following order:
1.  `GOOGLE_APPLICATION_CREDENTIALS_JSON`
2.  `GOOGLE_APPLICATION_CREDENTIALS`
3.  `../llm.json` when running from the `backend` directory

*Note: Never commit real credentials. Files like `llm.json`, `.env`, and `IDEA.md` are ignored by Git and Docker.*

### x402 Defaults targeting Algorand Testnet:
- Facilitator: `https://facilitator.goplausible.xyz`
- Network ID: `algorand:SGO1GKSzyE7IEPItTxCByw9x8FmnrCDexi9/cOUJOiI=`
- USDC ASA ID: `10458941`
- Default Amount: `1000` atomic units (or `0.001 USDC`)
- Receiving Wallet Address: configured via `X402_PAY_TO`

---

## Getting Started

### 1. Run the Backend
```sh
cd backend
cargo run
```
*The backend automatically loads the root `.env` file when started from the `backend` directory.*

### 2. Run the x402 Smoke Tests
```sh
./scripts/smoke_x402_dev.sh
```

### 3. Run a Real x402 Algorand Testnet Payment Test
```sh
cd frontend
npm run x402:pay -- "Which is the smartest cat?"
```
*This uses `AVM_MNEMONIC` from your `.env` file. The payer account must have TestNet ALGO, be opted in to USDC ASA `10458941`, and hold TestNet USDC.*

### 4. Build the Frontend
```sh
cd frontend
npm install
npm run build
```

*Default local URLs*:
*   Backend: `http://127.0.0.1:8080`
*   Frontend (Production build directory): `frontend/dist`

---

## Docker Support

### Build the Backend Image
```sh
docker build -t berlin42-answer-forge .
```

### Run the Docker Container
```sh
docker run --rm -p 8080:8080 \
  -e GOOGLE_CLOUD_PROJECT=<project-id> \
  -e GOOGLE_APPLICATION_CREDENTIALS_JSON='<service-account-json>' \
  berlin42-answer-forge
```
*The Docker image bundles `config_models.json` and the prompt master files. Secrets must be injected at runtime.*

---

## Project Structure

- `backend`: Rust API server and multi-model pipeline orchestration
- `frontend`: React/Vite web interface
- `prompts/master`: Multi-model prompt chain files
- `docs/architecture.md`: Architecture notes and system overview
- `docs/pipeline.md`: Detailed pipeline stage contract and schemas
- `infra/render.md`: Render cloud deployment guidelines
- `config_models.json`: Model configuration and routing file
