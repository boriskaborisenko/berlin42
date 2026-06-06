# NestorChat Backend

Rust API server for orchestrating the NestorChat pipeline.

## Development

During development, run the server directly from this folder:

```sh
cargo run
```

Docker is reserved for production-style builds and Render deployment.

## Smoke Tests

Mock pipeline:

```sh
curl -sS http://127.0.0.1:8080/api/runs \
  -H 'Content-Type: application/json' \
  -d '{"intent":"самый вкусный рецепт пирожка","artifact_type":"answer","variant_count":1,"research_enabled":false,"execution_mode":"mock"}'
```

Live initial pipeline:

```sh
curl -sS http://127.0.0.1:8080/api/runs \
  -H 'Content-Type: application/json' \
  -d '{"intent":"самый вкусный рецепт пирожка","artifact_type":"answer","variant_count":1,"research_enabled":false,"execution_mode":"live"}'
```

Read run snapshot:

```sh
curl -sS http://127.0.0.1:8080/api/runs/<run_id>
```

`POST /api/runs` returns quickly with `status: "running"`. The frontend polls `GET /api/runs/<run_id>` until the run becomes `completed` or `error`.

Real Vertex model:

```sh
curl -sS http://127.0.0.1:8080/api/test-model \
  -H 'Content-Type: application/json' \
  -d '{"model_id":"base_llm","prompt":"Ответь одним предложением: самый вкусный рецепт пирожка?"}'
```

Local credentials are loaded from `GOOGLE_APPLICATION_CREDENTIALS_JSON`, `GOOGLE_APPLICATION_CREDENTIALS`, or `../llm.json`. On Render, set credentials in the Dashboard environment variables.

## Planned Responsibilities

- expose JSON API endpoints
- validate pipeline requests
- orchestrate generation, review, red-team, merge, compression, eval, and revision
- provide mock provider adapters for local development
- later add OpenAI, Anthropic, and Vertex AI/Gemini adapters
- keep service account credentials out of the repository
