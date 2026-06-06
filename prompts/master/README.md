# Master Prompts

Universal prompt chain for NestorChat.

## Order

1. `00_base_brief.md`
2. `01_candidate_generation.md`
3. `02_cross_review.md`
4. `03_distributed_red_team.md`
5. `04_consensus_merge.md`
6. `05_compression.md`
7. `06_eval.md`
8. `07_revision.md`
9. `08_final_formatter.md`

## Product Contract

The user chooses:

- `skill`
- `prompt`
- `answer`

The pipeline first solves the request in a universal way, then the final formatter adapts the result to the requested artifact type.

The default output has two variants. The user can request three variants.
