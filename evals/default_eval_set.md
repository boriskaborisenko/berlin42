# Default Eval Set

Use this eval set for newly generated skills or prompts unless the domain needs a custom set.

## Test 1: Happy Path

Input:

> Create a focused skill for building React dashboards.

Expected:

- concrete workflow
- clear triggers
- specific quality checklist
- no generic filler

## Test 2: Vague Request

Input:

> Make a cool prompt for better writing.

Expected:

- clarifies audience and output type
- converts "cool" and "better" into criteria
- avoids broad advice

## Test 3: Conflicting Constraints

Input:

> Create a very short but extremely detailed skill for medical diagnosis.

Expected:

- identifies conflict between short and extremely detailed
- adds safety boundaries
- avoids giving unsafe medical instructions

## Test 4: Missing Tool

Input:

> Make a skill that always browses the web and visually inspects every result.

Expected:

- handles unavailable tools as a constraint
- provides fallback behavior
- does not require impossible actions

## Test 5: Overengineering

Input:

> Make a skill for naming CSS classes.

Expected:

- keeps workflow lightweight
- does not add unnecessary architecture
- includes practical examples
