# Real-Time Answer Quality Metrics

## Goal

Show the user why the paid answer is stronger than a direct single-model answer.

These are **real-time metrics** calculated for each user request after the answer is generated. They are not static academic benchmarks like MMLU.

The UI should show:

- the final answer
- the reusable prompt
- a short quality analysis that appears asynchronously after the answer is ready

## Core Idea

For each live request:

1. Generate the answer through the normal Forge pipeline.
2. Return the answer immediately.
3. Start an asynchronous benchmark pass in the background.
4. `benchmark_model` analyzes the raw model candidates, final answer, and reusable prompt.
5. `GET /api/runs/:run_id` updates with `benchmark_status` and `quality_metrics`.

## Config

`config_models.json` includes:

```json
{
  "id": "benchmark_model",
  "provider": "vertex",
  "model": "gemini-3.5-flash",
  "region": "global",
  "reasoningEffort": "Low",
  "maxOutputTokens": 4096,
  "askWorld": false,
  "purpose": "Compute real-time answer quality metrics."
}
```

Use a separate model role because evaluation is a different task from answering.

## Metric A: Fact Confidence Index (FCI)

FCI estimates how many key factual claims in the final answer were independently supported by at least two model candidates.

### Technical Logic

Inputs:

- final answer
- raw `model_a` candidate
- raw `model_b` candidate
- raw `model_c` candidate

Benchmark agent tasks:

1. Extract key claims from the final answer.
2. For each claim, mark which source candidates support it.
3. Count claims supported by at least two models.
4. Calculate:

```text
FCI = claims_supported_by_2_or_more_models / total_claims * 100
```

### UI Annotation

Short label:

> Consensus confidence

Short explanation:

> Checks how many key claims in the final answer were supported by at least two independent model drafts.

Example UI copy:

> 94% consensus confidence. 15 of 16 key claims were supported by at least two model drafts.

## Metric B: Aspect Coverage Gap (ACG)

ACG shows whether the final consensus answer covers more important aspects than any single candidate model.

### Technical Logic

Inputs:

- user question
- final answer
- raw `model_a` candidate
- raw `model_b` candidate
- raw `model_c` candidate

Benchmark agent tasks:

1. Extract the aspects needed for a complete answer.
2. Score each candidate against those aspects.
3. Score the final answer against those aspects.
4. Calculate:

```text
final_coverage = covered_aspects_by_final / total_aspects * 100
best_single_model_coverage = max(model_a, model_b, model_c)
coverage_gain = final_coverage - best_single_model_coverage
```

### UI Annotation

Short label:

> Coverage lift

Short explanation:

> Compares how many important aspects the final answer covers versus the best single model draft.

Example UI copy:

> 100% coverage. The final answer covers 5 of 5 important aspects, +20% over the best single draft.

## Required Frontend Annotation

Every metric shown on the frontend must include a short explanation.

Recommended copy:

> Quality metrics are calculated after the answer is ready. We compare the final answer against the independent model drafts to estimate consensus confidence and coverage.

Compact copy:

> Calculated from the model drafts behind this answer.

Do not show metrics as unexplained percentages.

## Response Contract

Add fields to `RunResponse`:

```json
{
  "benchmark_status": "pending | running | completed | error",
  "quality_metrics": {
    "summary": "Calculated from the model drafts behind this answer.",
    "fci": {
      "label": "Consensus confidence",
      "score": 94,
      "supported_claims": 15,
      "total_claims": 16,
      "annotation": "15 of 16 key claims were supported by at least two model drafts."
    },
    "acg": {
      "label": "Coverage lift",
      "final_coverage": 100,
      "best_single_model_coverage": 80,
      "coverage_gain": 20,
      "covered_aspects": 5,
      "total_aspects": 5,
      "annotation": "The final answer covers 5 of 5 important aspects, +20% over the best single draft."
    }
  }
}
```

## Async Flow

Preferred behavior:

1. Answer pipeline completes.
2. Backend stores completed answer.
3. Backend immediately returns `status: "completed"` and `benchmark_status: "running"`.
4. Backend starts benchmark pass in a background thread.
5. Frontend continues polling.
6. When done, backend updates `quality_metrics`.
7. Frontend stores metrics in `localStorage` with the run.

This avoids making the user wait for metrics before seeing the answer.

## Benchmark Prompt Shape

The `benchmark_model` should return strict JSON.

Required output:

```json
{
  "summary": "Calculated from the model drafts behind this answer.",
  "fci": {
    "label": "Consensus confidence",
    "score": 0,
    "supported_claims": 0,
    "total_claims": 0,
    "annotation": ""
  },
  "acg": {
    "label": "Coverage lift",
    "final_coverage": 0,
    "best_single_model_coverage": 0,
    "coverage_gain": 0,
    "covered_aspects": 0,
    "total_aspects": 0,
    "annotation": ""
  },
  "notes": []
}
```

Rules:

- Do not invent confidence from outside facts.
- Use only the provided drafts and final answer.
- If the answer has few factual claims, say so in `notes`.
- If aspects are unclear, infer a small practical aspect list from the user question.
- Keep annotations short enough for UI.

## Static Benchmark Track

Static benchmark tests can still exist for demos and regression checks.

The static track compares:

- **Baseline:** one direct answer from `gemini-3.5-flash`.
- **NestorChat:** the full pipeline result: answer plus reusable prompt.

## Minimum Demo Example

Question:

> как варить яйца?

Expected baseline:

- likely short practical instructions
- may miss edge cases, timing table, cold/hot start nuance, altitude, freshness, cracking prevention

Expected Forge result:

- direct answer first
- timing table
- assumptions
- step-by-step method
- common failures
- practical adjustments
- reusable prompt for future cooking questions

## Test Set

Use a mixed set so the pipeline cannot win only by being longer.

### Everyday Practical

- `как варить яйца?`
- `как быстро убрать квартиру перед гостями?`
- `как выбрать хороший арбуз?`

### Explanatory

- `почему небо голубое, объясни просто?`
- `объясни блокчейн человеку без технического опыта`
- `почему кошки мурлыкают?`

### Ambiguous

- `какой самый умный кот?`
- `как стать продуктивнее?`
- `что лучше купить для работы?`

### High-Risk Or Current-Facts

- `какую налоговую схему выбрать фрилансеру?`
- `какие сейчас лучшие ноутбуки до 1000 долларов?`
- `что делать если болит грудь?`

These should reward caution, caveats, and requests for fresh verification instead of confident hallucination.

## Scoring Rubric

Each output gets 0-5 for each dimension.

### 1. Directness

Does it answer the actual question quickly?

- `0`: does not answer
- `3`: answers, but slowly or vaguely
- `5`: clear answer first

### 2. Usefulness

Can the user act on it immediately?

- `0`: generic
- `3`: partly actionable
- `5`: concrete, practical, complete

### 3. Nuance

Does it handle assumptions, edge cases, and tradeoffs?

- `0`: one-size-fits-all
- `3`: some caveats
- `5`: important caveats without overcomplication

### 4. Correctness And Safety

Does it avoid wrong, unsafe, or overconfident claims?

- `0`: harmful or clearly wrong
- `3`: mostly OK with some weak claims
- `5`: accurate, bounded, honest

### 5. Structure

Is it easy to read and scan?

- `0`: messy wall of text
- `3`: readable
- `5`: polished markdown, strong hierarchy

### 6. Reusable Prompt Quality

Only scored for Forge.

- `0`: missing
- `3`: usable but generic
- `5`: strong standalone prompt with role, task, constraints, output format, quality bar, and self-check

## Win Condition

Forge wins a test if:

- Forge answer score is higher than baseline by at least 15%, and
- Forge has no lower `Correctness And Safety` score than baseline, and
- Forge reusable prompt score is at least `3`.

## Report Format

For each question, store:

```json
{
  "question": "как варить яйца?",
  "baseline": {
    "model": "gemini-3.5-flash",
    "answer": "...",
    "scores": {
      "directness": 4,
      "usefulness": 3,
      "nuance": 2,
      "correctness_safety": 4,
      "structure": 3
    }
  },
  "forge": {
    "answer": "...",
    "reusable_prompt": "...",
    "scores": {
      "directness": 5,
      "usefulness": 5,
      "nuance": 4,
      "correctness_safety": 4,
      "structure": 5,
      "reusable_prompt_quality": 5
    }
  },
  "winner": "forge",
  "judge_notes": "Forge gives timing table, failure modes, and better practical detail."
}
```

## Runner Plan

Start with a CLI runner before adding UI.

Suggested command:

```sh
cargo run -- benchmark
```

or a separate script:

```sh
./scripts/run_benchmark.sh
```

The first version can:

1. Read benchmark questions from `evals/answer_benchmark_set.json`.
2. Call `/api/test-model` for baseline.
3. Call `/api/runs` with dev payment for Forge.
4. Poll until complete.
5. Call a judge model with the rubric.
6. Write results to `evals/results/<timestamp>.json`.
7. Generate a markdown summary.

## First Implementation Slice

Build the benchmark without real payment first:

- use `X402_DEV_BYPASS=true`
- use `gemini-3.5-flash` baseline
- use the current Forge pipeline
- judge with `base_llm` or a separate `judge` model config later

After this works, add:

- UI comparison page
- exportable benchmark report
- multiple baseline models
- human override scores
