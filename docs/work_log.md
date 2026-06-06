# Work Log

## 2026-06-06

### Decisions

- Product direction changed from generic skill/prompt/answer generation to NestorChat.
- Hackathon flow is now payment-first: x402 on Algorand testnet gates live AI runs.
- Final product output should include both a polished answer and a reusable prompt.
- Work order: x402 payment gate, debate pipeline quality, benchmarks, then UI polish.
- `llm.json`, `.env`, and `IDEA.md` are local-only and must stay ignored.

### Verified

- `cargo check` passes for the backend after README/Docker hygiene changes.
- `.gitignore` ignores `llm.json`, `IDEA.md`, backend target, frontend node modules, and frontend dist.
- Dockerfile now copies `prompts/master` and sets `PROMPTS_DIR`, which live mode needs at runtime.

### Current Plan

See `docs/roadmap.md`.

### Next Action

Design and implement the x402/Algorand payment verification contract before allowing live runs.

## 2026-06-04

### Decisions

- Project starts as a documentation-first prototype.
- MVP will prove the full pipeline with mock providers before requiring real model keys.
- Real `.env` files are ignored; committed config uses `.env.example`.
- Vertex AI service account data should be provided through Render environment variables, not committed files.
- `llm.json` is treated as a local service account credential and must stay ignored.
- Backend and frontend live in separate top-level folders.
- Docker is for production-style deployment; local development runs the Rust server directly.
- A project-local skill lives at `skills/skill-creator-project/SKILL.md`.
- User-facing output modes are `skill`, `prompt`, and `answer`.
- Final output returns two variants by default and can return three variants on request.
- Backend has two config layers: ENV for secrets/runtime and `config_models.json` for non-secret model routing.
- Render ENV values must be entered in the Render Dashboard, not committed as `.env`.
- Default test models are Gemini-only, all `global` region.
- Debate uses three participant models and no separate judge model by default.
- Model config includes `reasoningEffort` as a non-secret quality/cost control.
- Backend has `/api/test-model` for a single real Vertex/Gemini smoke test before wiring the full debate pipeline to live calls.
- Universal master prompt set added under `prompts/master/`.

### Verified

- `cargo fmt` passes for backend.
- `cargo check` passes for backend.
- Local credentials from `../llm.json` can authenticate with Vertex AI.
- `POST /api/test-model` works for `base_llm` / `gemini-2.5-flash` in `global`.
- `POST /api/test-model` works for `model_c` / `gemini-2.5-pro` in `global`.
- `finish_reason` is exposed in test responses; increasing `maxOutputTokens` to `2048` fixed early `MAX_TOKENS` truncation in the smoke test.
- `POST /api/runs` supports `execution_mode: "live"` for real base brief and real A/B/C candidate generation.
- Model config supports `maxOutputTokens` as a number or `"unlimited"`; `"unlimited"` omits the Vertex `maxOutputTokens` field.
- Live initial pipeline verified: `base_llm` produced a shared task brief and A/B/C produced live candidates with `finish_reason: STOP`.
- Live A/B/C candidate generation now runs in parallel after the shared `base_llm` brief.
- Local test config currently reuses `gemini-3.5-flash` for `base_llm`, `model_a`, `model_b`, and `model_c`.
- Final formatting now returns artifact-shaped outputs: a real skill for `skill`, a reusable prompt for `prompt`, and a same-language response for `answer`.
- `POST /api/runs` now creates a background job and returns `running`; `GET /api/runs/:run_id` returns snapshots for polling.
- Frontend always submits live runs, polls the run snapshot, and shows active/done/waiting states in the pipeline rail.
- Live final formatting now calls the configured finalizer model and synthesizes one finished artifact when `variant_count` is `1`.
- Live smoke test verified one Russian final `answer` without copied candidate variant sections.
- Product name changed to `NestorChat`.
- Frontend stack is React + Vite, not Next.js, because Rust remains the API/orchestration server.
- UI now models multi-user, multi-run state with `user_id`, `run_id`, stage progress, and SSE-ready presentation.
- `UI_SKILL.md` updated to use Google Sans, Google Sans Flex, and Google Sans Code font guidance.
- UI simplified to request one final result by default; variant selector removed from frontend.

### Current Plan

1. Create project docs and pipeline specification. Done.
2. Add environment and repository hygiene files. Done.
3. Scaffold Rust backend and React frontend. Done.
4. Implement mock pipeline end to end. Done.
5. Implement live initial pipeline: base brief plus A/B/C generation. Done.
6. Implement live cross-review. Next.
7. Add provider adapter interfaces.
8. Add real provider integrations one by one.
9. Add Docker and Render deployment.
10. Implement async run storage and `GET /api/runs/:id/events` SSE.

### Open Questions

- Which Rust web framework should be used: Axum, Actix Web, or Rocket?
- Should runs be persisted in SQLite/Postgres or kept in memory for MVP?
- Which web research provider should be used?
- Should `answer` mode use the full expensive pipeline every time or allow a faster lightweight path?

### Next Action

Choose the first vertical slice and scaffold the app around mock providers.
