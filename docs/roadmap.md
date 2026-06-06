# Berlin42 Roadmap

## Product Goal

Berlin42 Answer Forge is a paid multi-model answer service for the hackathon. A user pays for a request through x402 on Algorand testnet, submits a question or task, and receives:

- a polished final answer
- a reusable prompt they can paste into any LLM
- transparent pipeline progress without exposing noisy internal debate

## Phase 1: x402 Payment Gate

Goal: a live AI run starts only after a verified testnet payment.

Backend tasks:

- Define payment config in env: network, recipient, amount, asset, timeout, dev bypass. Done.
- Add a payment requirement endpoint or response shape for unpaid requests. Done.
- Add payment verification before `POST /api/runs` starts live execution. Done.
- Reject missing, invalid, reused, expired, or wrong-recipient payments.
- Keep mock/dev mode available without real payment. Done.

Frontend tasks:

- Add payment state before run submission.
- Show amount, network, recipient, and payment status.
- Pass payment proof/receipt to the backend.

Acceptance criteria:

- Unpaid live request cannot start the pipeline.
- Valid Algorand testnet payment starts a run.
- Invalid or reused payment is rejected.
- Local development can still run with explicit dev bypass.

## Phase 2: Debate Pipeline Upgrade

Goal: make the paid answer meaningfully better than a single model response.

Backend tasks:

- Wire live cross-review for `model_a`, `model_b`, and `model_c`.
- Wire live distributed red-team.
- Implement deterministic consensus merge.
- Implement compression, eval, and revision stages.
- Change the final response contract to return both `answer` and `reusable_prompt`.
- Keep stage snapshots available through `GET /api/runs/:run_id`.

Prompt tasks:

- Tune master prompts for premium paid answers.
- Make final formatter produce a direct answer and portable prompt.
- Prevent final output from mentioning internal model A/B/C debate unless requested.

Acceptance criteria:

- One paid request returns one strong answer and one copyable prompt.
- Pipeline stages are inspectable.
- Final answer is concrete, same-language, and free of internal pipeline noise.

## Phase 3: Benchmarks

Goal: prove the debate pipeline is better than a single baseline model and show per-answer quality metrics in the UI.

Tasks:

- Create benchmark set in `evals/`. Started in `evals/answer_benchmark_set.json`.
- Add real-time metric role in `config_models.json`. Done: `benchmark_model`.
- Add async per-run metrics: Fact Confidence Index and Aspect Coverage Gap.
- Return short benchmark annotations so frontend percentages are understandable.
- Add baseline runner: direct single-model answer.
- Add pipeline runner: full debate answer.
- Define scoring rubric: correctness, usefulness, specificity, hallucination risk, prompt quality.
- Store benchmark inputs, outputs, and scores.

Acceptance criteria:

- Benchmarks can run locally.
- Results compare baseline vs pipeline.
- Failure cases are visible enough to guide prompt/pipeline tuning.

## Phase 4: UI

Goal: make the hackathon demo feel like a premium paid answer product.

Tasks:

- Design the first screen around request plus payment.
- Add states: payment required, verifying, running, completed, payment failed, run failed.
- Show final answer and reusable prompt as first-class outputs.
- Add transaction/proof visibility for the demo.
- Polish pipeline progress without exposing confusing internals.

Acceptance criteria:

- Demo flow is understandable in 30 seconds.
- User clearly sees payment, processing, answer, and prompt.
- UI no longer feels like an internal pipeline debugger.

## Immediate Next Task

Start Phase 1 by designing the exact x402/Algorand backend contract:

- request shape
- verification response shape
- env variables
- dev bypass behavior
- where payment proof attaches to `/api/runs`
