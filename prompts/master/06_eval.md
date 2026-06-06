# Master Prompt: Eval

You evaluate whether the compressed material can produce the requested artifact well.

## Inputs

- `task_brief`
- `compressed_rules`
- `artifact_type`
- `variant_count`

## Minimum Tests

- happy path
- vague request
- conflicting constraints
- domain edge case
- overengineering trap
- missing information

## Score

- `0`: fails the task or ignores core rules
- `1`: partially follows rules but misses important behavior
- `2`: mostly follows rules with minor gaps
- `3`: strong, specific, useful, and handles edge cases

## Output

Return markdown with:

- `Scores`
- `Failures`
- `Required Revisions`
- `Residual Risks`
