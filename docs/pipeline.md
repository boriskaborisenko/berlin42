# NestorChat Pipeline Specification

## Output Modes

The pipeline supports three artifact modes:

- `skill`: generates a Codex-style `SKILL.md`.
- `prompt`: generates a reusable prompt.
- `answer`: generates a direct answer to the user's request, but still uses generation, critique, red-team, merge, compression, eval, and revision.

The pipeline also supports final variants:

- default: `variant_count = 2`
- optional: `variant_count = 3`
- each variant should be meaningfully different, not a paraphrase

## Stage Contract

Every stage should produce structured output:

```json
{
  "stage": "generation",
  "model": "openai",
  "artifact_type": "skill",
  "variant_index": null,
  "summary": "Short human-readable summary",
  "content": "Markdown artifact or review",
  "findings": [],
  "score": null
}
```

## Master Prompt Set

The backend should use universal master prompts from `prompts/master/`:

- `00_base_brief.md`: turns the raw user request into a shared task brief.
- `01_candidate_generation.md`: asks A/B/C to produce independent candidates.
- `02_cross_review.md`: asks each model to critique the other two.
- `03_distributed_red_team.md`: asks each model to stress-test the whole debate.
- `04_consensus_merge.md`: extracts merge-ready consensus material without a judge model.
- `05_compression.md`: converts merged material into executable rules.
- `06_eval.md`: tests the compressed material.
- `07_revision.md`: fixes eval failures.
- `08_final_formatter.md`: adapts the final output to `skill`, `prompt`, or `answer`.

These prompts are universal. The final formatter decides whether the result becomes a complete skill, a reusable prompt, or a long understandable answer.

## Model Debate Architecture

The pipeline is designed as a controlled argument, not a simple fan-out. The default architecture avoids a fourth judge model.

1. `base_llm` receives the user's raw request and builds a unique shared task brief.
2. The backend gives the same task brief to `model_a`, `model_b`, and `model_c`.
3. A/B/C independently generate candidates.
4. A/B/C cross-review each other.
5. A/B/C each red-team the whole debate.
6. The backend uses deterministic consensus merge rules to prepare merge material. This is not a judge model.
7. The merged material is compressed into executable rules.
8. Eval tests whether the rules can satisfy the task.
9. Revision fixes failures found by eval.
10. Final formatter optimizes the result for `skill`, `prompt`, or `answer`.

The final variants are conclusions from the debate. They should expose different useful strategies, not merely different wording.

A fourth judge model can be added later as an optional mode if deterministic consensus is not good enough, but it is not part of the default pipeline.

## Stage 0: Base Prompt Builder

Input:

- raw user intent
- artifact mode
- final variant count
- user constraints
- optional research notes

Output:

- target audience
- target artifact format
- final variant strategy
- success criteria
- constraints
- required sections
- evaluation tasks
- shared task brief
- model instructions

Rules:

- Convert vague intent into concrete acceptance criteria.
- Preserve user constraints.
- Add missing quality dimensions relevant to the domain.
- Mark assumptions explicitly.
- Do not invent fake citations.

## Stage 1: Generation

Models independently generate candidates from the shared task brief.

Required candidate sections for `skill` mode:

- purpose
- when to use
- always do
- never do
- workflow
- quality checklist
- examples
- tests
- failure criteria

Required candidate sections for `prompt` mode:

- role/context
- input contract
- workflow
- output contract
- constraints
- quality bar
- self-check
- examples

Required candidate behavior for `answer` mode:

- answer the user directly
- state assumptions
- preserve useful structure
- avoid unnecessary prompt/skill scaffolding
- include verification or next-step criteria when relevant

## Stage 2: Cross-review

Each model reviews the other candidates.

Review criteria:

- specificity
- executability
- contradictions
- missing edge cases
- domain completeness
- eval readiness
- overengineering risk
- safety or privacy risk

Reviews must include:

- concrete issue
- affected candidate
- severity: low, medium, high
- proposed fix

## Stage 3: Distributed Red-team

Each debate model attacks the combined candidate set.

It must search for:

- vague words that do not change behavior
- conflicting instructions
- impossible instructions
- missing failure modes
- hidden dependency on unavailable tools
- harmful or unsafe guidance
- instructions that make outputs worse

## Stage 4: Consensus Merge

The consensus merge creates a master artifact and then derives the requested final variants.

Rules:

- Prefer concrete rules over broad advice.
- Convert review findings into explicit instructions.
- Remove duplicated sections.
- Keep the artifact usable by an LLM in one pass.
- Preserve examples only if they teach a rule.
- If `variant_count` is greater than one, variants must differ by approach, depth, or format.
- Never create variants that are only reworded copies.

## Stage 5: Compression

Compression is rule extraction, not summarization.

Keep:

- mandatory behaviors
- forbidden behaviors
- ordered workflow
- checks
- examples that prevent common failure
- eval criteria

Remove:

- motivational prose
- repeated advice
- generic best-practice language
- obvious statements
- model-flattery or self-description

## Stage 6: Eval

Eval runs the artifact against test tasks.

Minimum eval set:

- happy path task
- vague user request
- conflicting constraints
- domain edge case
- request that should be refused or narrowed

Eval scoring:

- `0`: fails or ignores skill/prompt
- `1`: partially follows but misses important rules
- `2`: follows most rules with minor gaps
- `3`: strong output, follows rules, handles edge cases

## Stage 7: Revision

Revision input:

- compressed artifact
- eval outputs
- eval failures
- red-team findings

Revision output:

- final variants
- changelog
- unresolved risks

Rules:

- Every high-severity eval failure needs a fix or explicit unresolved-risk note.
- Do not add large new sections unless they prevent repeated failure.
- Re-run compression after revision.

## Stage 8: Final Formatting

The final formatter adapts the revised rules to the requested artifact type.

For `skill`, output complete `SKILL.md` candidates.

For `prompt`, output reusable prompts.

For `answer`, output long, clear, user-facing answers.

Rules:

- Produce exactly the requested number of variants.
- Hide internal debate noise.
- Preserve assumptions and caveats that matter.
- Make variants meaningfully different.
