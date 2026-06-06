# TriForge Architecture Plan

## Product Understanding

TriForge is a universal factory for LLM skills, strong prompts, and long-form answers. It turns a rough user request into a tested, compact, executable artifact through generation, critique, red-team review, merge, compression, eval, and revision.

The first version should favor traceability over automation magic: the user should be able to see what each stage produced and why the final artifact changed.

## Assumptions

- The backend will be a Rust service exposing JSON APIs.
- The frontend will be React.
- The app will run locally with Docker and deploy to Render as a Docker service.
- Real model providers will be pluggable behind provider adapters.
- Development can begin with mock providers so the pipeline can be tested without paid API calls.
- Vertex AI credentials will be passed through environment variables in Render and local `.env`, never committed.

## Non-goals For MVP

- Multi-user accounts.
- Billing.
- Persistent project workspaces.
- Full browser automation for visual QA.
- Perfect provider parity across OpenAI, Claude, and Gemini.
- Automatic publishing of generated skills.

## System Components

### Frontend

React app for:

- entering the raw intent
- selecting output type: skill, prompt, or answer
- selecting final variant count: two by default, three on request
- toggling web research
- running the pipeline
- inspecting each stage output
- editing or accepting the final artifact
- copying/exporting final markdown

### Backend

Rust API server for:

- request validation
- pipeline orchestration
- provider adapter calls
- web research abstraction
- stage output normalization
- eval execution
- final artifact assembly
- loading runtime ENV and model config separately

## Configuration Layers

TriForge uses two configuration layers.

### ENV

ENV is for secrets and deployment/runtime values:

- app host and port
- provider API keys
- Vertex AI project and location
- service account JSON
- optional `CONFIG_MODELS_PATH`

Local development may use a local `.env` file, but it must never be committed.

On Render, set these values directly in the Render Dashboard environment variable UI. Do not commit a production `.env` file and do not bake secrets into Docker.

### `config_models.json`

`config_models.json` is for non-secret model behavior:

- `id`: pipeline role, for example `base_llm`, `model_a`, `red_team`, or `merge`
- `provider`: `openai`, `anthropic`, `vertex`, or `mock`
- `model`: provider model name
- `region`: `global`, `us-central1`, or another provider-supported region
- `reasoningEffort`: reasoning budget hint such as `Low`, `Medium`, `High`, or `XHigh`
- `maxOutputTokens`: either a number or `"unlimited"`; `"unlimited"` means the backend omits `maxOutputTokens` and lets the provider apply its own limit
- `askWorld`: whether this model is allowed to use internet/research tools when the run requests research
- `purpose`: short description of the model's pipeline role

The default test config uses four Gemini models, all in `global` region:

- `base_llm`: `gemini-2.5-flash`
- `model_a`: `gemini-3.5-flash`
- `model_b`: `gemini-3.1-pro-preview`
- `model_c`: `gemini-2.5-pro`

Only `model_a`, `model_b`, and `model_c` are debate participants. `base_llm` builds the brief. The backend performs deterministic consensus merge, so there is no separate judge model by default.

Effective internet access is conservative:

```text
effective_ask_world = request.research_enabled && model.askWorld
```

This lets us deploy the same code safely while changing model choices without touching secrets.

### Provider Layer

Provider adapters expose a common interface:

- `generate_candidate`
- `review_candidates`
- `red_team`
- `merge`
- `compress`
- `evaluate`
- `revise`

Initial adapters:

- `mock`
- `vertex_gemini`

### Research Layer

The research layer should be optional. The pipeline should work without network access.

Research output should be normalized into short evidence notes:

- source title
- source URL
- key takeaway
- relevance to the target skill/prompt

### Storage

MVP can be stateless or file-based. Persisting runs is useful but not required for the first vertical slice.

If persistence is added, use a simple `runs` table and store stage outputs as JSON.

## API Draft

### `POST /api/runs`

Creates and starts a pipeline run.

Request:

```json
{
  "intent": "Create a skill for website design",
  "artifact_type": "skill",
  "variant_count": 2,
  "execution_mode": "mock",
  "research_enabled": true,
  "providers": ["openai", "anthropic", "vertex_gemini"]
}
```

Response:

```json
{
  "run_id": "run_123",
  "status": "completed",
  "artifact_type": "answer",
  "variant_count": 3,
  "stages": [],
  "final_variants": []
}
```

`execution_mode`:

- `mock`: default, no real provider calls
- `live`: real `base_llm` brief building and real A/B/C candidate generation; later stages currently remain deterministic placeholders

### `GET /api/models/config`

Returns the active non-secret model config loaded by the backend.

### `POST /api/test-model`

Calls one real Vertex/Gemini model using service account credentials.

Request:

```json
{
  "model_id": "base_llm",
  "prompt": "Ответь одним предложением: самый вкусный рецепт пирожка?"
}
```

Local credentials are resolved in this order:

1. `GOOGLE_APPLICATION_CREDENTIALS_JSON`
2. `GOOGLE_APPLICATION_CREDENTIALS`
3. `../llm.json` when running from `backend`

Render should use Dashboard environment variables, especially `GOOGLE_APPLICATION_CREDENTIALS_JSON` and `GOOGLE_CLOUD_PROJECT`.

### `GET /api/runs/:id`

Returns run status and stage outputs.

### `POST /api/runs/:id/revise`

Applies user feedback to the final artifact and reruns revision/compression/eval.

## Environment Variables

Core:

- `APP_ENV`
- `APP_HOST`
- `APP_PORT`
- `RUST_LOG`

Provider keys:

- `OPENAI_API_KEY`
- `ANTHROPIC_API_KEY`
- `GOOGLE_CLOUD_PROJECT`
- `GOOGLE_CLOUD_LOCATION`
- `GOOGLE_APPLICATION_CREDENTIALS_JSON`

Optional:

- `RESEARCH_PROVIDER`
- `RESEARCH_API_KEY`

## Render Deployment Notes

Render should receive secrets through Dashboard environment variables.

Do not bake secrets into the Docker image.

Render should not depend on a committed `.env` file. The Docker image includes `config_models.json`, while secrets are injected at runtime.

For Vertex AI service account JSON, prefer one Render environment variable:

- `GOOGLE_APPLICATION_CREDENTIALS_JSON`

The backend can write this value to a temporary credentials file at startup if the Google SDK requires a file path.

## Security Rules

- Never log full prompts if they include user secrets.
- Never log API keys or service account JSON.
- Reject oversized requests.
- Treat web research content as untrusted input.
- Keep provider errors sanitized in frontend responses.
