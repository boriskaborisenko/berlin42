---
name: skill-creator-project
description: Use when working inside the Skill Creator project: a Rust backend, React frontend, Docker/Render deployment, and multi-model pipeline for generating high-quality LLM skills and reusable prompts.
---

# Skill Creator Project

## Purpose

Guide development of the Skill Creator app without losing the product shape: a universal factory for strong skills, prompts, and direct answers built through generation, cross-review, red-team, merge, compression, eval, and revision.

## Always Do

- Keep secrets out of Git. Treat `llm.json`, `.env`, service account JSON, and API keys as private.
- Prefer mock providers before real provider integrations so the whole pipeline can run locally.
- Keep the Rust backend as the orchestration authority.
- Keep the React frontend focused on input, visibility, stage inspection, and final artifact review.
- Preserve traceability: every final artifact should be explainable from stage outputs.
- Preserve the product contract: output mode is `skill`, `prompt`, or `answer`; final variants default to two and can be three on request.
- Update `docs/work_log.md` when architectural decisions or scope change.
- Make generated skills executable: clear triggers, always/never rules, workflow, checklist, examples, tests, and failure criteria.

## Never Do

- Never commit real credentials or service account JSON.
- Never bake secrets into Docker images.
- Never make web research mandatory for local development.
- Never let model-specific logic leak through the whole backend; keep providers behind adapters.
- Never accept vague skill output as final if it lacks tests or failure criteria.
- Never overbuild distant features before the first vertical slice works end to end.

## Development Workflow

1. Read `IDEA.md`, `docs/architecture.md`, and `docs/pipeline.md` before changing product behavior.
2. For backend work, define the data contract first, then implement the Rust route or pipeline module.
3. For frontend work, match the API contract and show stage state clearly.
4. For AI work, start with prompt assets in `prompts/` and deterministic mock outputs.
5. Verify with the smallest useful test: unit test for pure pipeline logic, API smoke test for routes, UI smoke test for frontend flows.
6. Update docs only where the decision affects future work.

## Pipeline Quality Bar

Generated artifacts must include:

- concrete always and never rules
- ordered workflow
- quality checklist
- examples of weak and strong behavior where useful
- eval tasks
- failure criteria
- compression that removes fluff but preserves behavior-changing rules

## Failure Criteria

Work in this project fails if it:

- exposes secrets
- makes local development depend on paid APIs
- hides intermediate model reasoning from the user-facing run trace
- produces generic skills or prompts
- skips eval and revision for final artifacts
