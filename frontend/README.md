# Berlin42 Answer Forge

Berlin42 Answer Forge is a multi-agent answer refinement engine.

It takes one user request and turns it into a stronger, safer, more complete answer by passing it through a structured reasoning pipeline: decomposition, multiple candidate generations, critique, red-team review, judge-model evaluation, consensus merging, and final answer polishing.

The core idea is simple:

> A good answer should not be produced by one model in one shot.  
> It should be forged through competing drafts, criticism, verification, and synthesis.

---

## Core Idea

Most LLM applications treat the model response as the final product.

Berlin42 treats the first response as raw material.

A single model can be useful, but it has obvious weaknesses:

- it can miss important angles;
- it can answer too confidently;
- it can ignore ambiguity;
- it can hallucinate details;
- it can optimize for fluency instead of correctness;
- it rarely explains what was uncertain;
- it does not check itself hard enough.

Answer Forge is built around the opposite assumption:

> The value is not in asking one model once.  
> The value is in building a process that makes the answer survive criticism.

---

## What the System Does

Berlin42 receives a user question and produces two main artifacts:

1. Final Answer — a polished, structured answer after several reasoning and review stages.
2. Reusable Prompt — a refined prompt that captures the logic of the answer and can be reused in another LLM.

The system is not just a chatbot wrapper. It is a controlled answer-production pipeline.

The pipeline can be extended with different reasoning roles:

- planner model;
- candidate models;
- critic model;
- red-team model;
- judge model;
- consensus model;
- final formatter;
- prompt extractor.

Each role has a separate responsibility.

---

## Why Multi-Step Answer Forging Matters

The main problem with simple LLM answers is not that they are always wrong.

The problem is that the user often cannot see:

- what was checked;
- what was assumed;
- what was uncertain;
- what alternatives were considered;
- what parts of the answer are strongly supported;
- what parts are fragile.

Berlin42 makes the answer pass through a repeatable process before presenting it as final.

Instead of:

txt User question → One model → Final answer 

Berlin42 uses:

txt User question → task interpretation → candidate answers → critique → red-team review → judge evaluation → consensus merge → final answer → reusable prompt 

The goal is not to make the answer longer.

The goal is to make it more reliable.

---

## Pipeline

### 1. Task Interpretation

The system first converts the raw user request into a clear task brief.

This stage identifies:

- what the user is actually asking;
- hidden assumptions;
- ambiguity;
- required output format;
- risk level;
- whether facts may need verification;
- what a good answer should contain.

This prevents later models from solving slightly different versions of the same task.

---

### 2. Candidate Generation

Multiple independent candidate answers are generated from the same task brief.

Each candidate can approach the problem differently:

- one can be practical;
- one can be analytical;
- one can be skeptical;
- one can be concise;
- one can focus on edge cases;
- one can focus on structure.

This gives the system raw diversity.

The point is not to average the candidates.

The point is to expose more possible reasoning paths before choosing the strongest one.

---

### 3. Critique Layer

A critic model reviews the candidate answers and looks for weaknesses.

It checks:

- missing points;
- unsupported claims;
- contradictions;
- vague language;
- overconfident statements;
- poor structure;
- weak reasoning;
- bad prioritization;
- useless filler.

The critic does not write the final answer.

Its job is to damage weak drafts before they reach the final stage.

---

### 4. Red-Team Layer

The red-team model attacks the answer from an adversarial perspective.

It asks questions like:

- What could be wrong here?
- What assumption is hidden?
- What would fail in practice?
- What did the candidates ignore?
- Where is the answer too confident?
- What would a skeptical expert object to?
- What could mislead the user?

This layer is especially important for:

- technical explanations;
- research tasks;
- business analysis;
- legal or financial topics;
- medical or safety-sensitive topics;
- ambiguous company/person/entity research;
- current or fast-changing information.

The red-team layer is not there to be polite.

It exists to find cracks.

---

### 5. Judge Model

The judge model evaluates candidate answers and critique results.

It can score or compare outputs across dimensions such as:

- correctness;
- completeness;
- usefulness;
- clarity;
- uncertainty handling;
- factual support;
- structure;
- directness;
- actionability;
- hallucination risk.

The judge model helps decide which parts of which candidates should survive.

It can also reject an answer if the available evidence is too weak.

---

### 6. Consensus Merge

The consensus layer merges the strongest surviving parts.

It prefers claims that are:

- independently supported by several candidates;
- not successfully attacked by the red-team layer;
- useful to the user;
- clear enough to include;
- properly caveated when uncertain.

The consensus model is not a summarizer.

It is a selector and synthesizer.

Its job is to preserve value and remove fragile reasoning.

---

### 7. Final Answer Formatter

The final formatter turns the merged reasoning into a clean user-facing answer.

It optimizes for:

- clarity;
- structure;
- directness;
- readability;
- practical usefulness;
- honest uncertainty;
- no unnecessary internal noise.

The final user should not see the entire internal debate.

They should see the result of that debate.

---

### 8. Reusable Prompt Extraction

After the answer is forged, the system generates a reusable prompt.

This prompt captures:

- the role;
- the task;
- the reasoning process;
- the checks;
- the structure;
- the constraints;
- the quality criteria.

The reusable prompt is valuable because it lets the user repeat the same kind of reasoning later.

The final answer solves the current problem.

The reusable prompt gives the user a tool for future problems.

---

## Example

A user asks:

txt Is this company legit and worth working with? 

A simple LLM answer may give a fluent opinion.

Berlin42 should instead:

1. identify ambiguity in the company name;
2. separate confirmed facts from assumptions;
3. generate several independent assessments;
4. red-team the business risks;
5. check whether claims are supported;
6. flag missing evidence;
7. produce a careful final answer;
8. generate a reusable due-diligence prompt.

The output should not pretend to know what it cannot know.

A strong answer may say:

txt There are several entities with similar names. Based on the available information, the safest conclusion is not “good” or “bad”, but “not enough verified evidence yet”. Here is what is known, what is unclear, and what should be checked before trusting the company. 

That is better than a confident hallucination.

---

## What Makes This Different

Berlin42 is not trying to be another chat UI.

The important part is the answer-production logic.

The value is in the pipeline:

txt Generate → Criticize → Attack → Judge → Merge → Polish → Extract Prompt 

This creates room for improving the system without changing the user interface.

The project can be improved by tuning:

- model roles;
- critique prompts;
- red-team prompts;
- judge criteria;
- consensus rules;
- scoring logic;
- confidence thresholds;
- retry conditions;
- answer formats;
- prompt extraction logic.

The architecture allows the answer logic to become smarter over time.

---

## Answer Quality Philosophy

Berlin42 follows a few basic principles.

### 1. Do not trust the first answer

The first model output is treated as a draft, not as truth.

### 2. Disagreement is useful

If candidate models disagree, the system should expose the conflict internally and resolve it carefully.

### 3. Uncertainty is part of quality

A good answer should clearly separate:

- known facts;
- likely interpretations;
- assumptions;
- missing evidence;
- speculation.

### 4. Criticism improves output

The system should intentionally search for flaws before finalizing the answer.

### 5. The final answer should be simple

Complexity belongs inside the pipeline.

The user-facing result should be clean.

---

## Current Implementation

The current implementation includes:

- Rust backend;
- React/Vite frontend;
- asynchronous run execution;
- run polling;
- model configuration through config_models.json;
- Vertex/Gemini integration;
- mock mode for local testing;
- x402 payment-gated requests;
- Algorand Testnet payment support;
- answer and reusable prompt output;
- benchmark-oriented quality layer.

The payment layer is not the whole product.

Payment is only the access mechanism.

The core product is the answer-forging process.

---

## x402 Layer

Berlin42 uses x402 as a way to gate expensive AI computation.

The logic is:

txt No payment → no expensive multi-model run Valid payment → answer pipeline starts 

This is useful because a forged answer may require several model calls instead of one.

x402 makes that computation accessible as a paid API primitive.

The important part is not “pay to chat”.

The important part is:

> Pay to run a structured answer-quality pipeline.

---

## Quality Metrics

Berlin42 can evaluate forged answers using internal quality metrics.

Possible metrics include:

### Consensus Support

How many key claims are supported by more than one candidate?

### Coverage Lift

Does the final answer cover more important aspects than the best individual draft?

### Red-Team Survival

Which claims survived adversarial review?

### Judge Score

How does the judge model score the final answer across correctness, usefulness, clarity, and uncertainty handling?

### Prompt Reusability

Is the generated prompt actually useful for reproducing the reasoning process?

These metrics should not be decorative.

They should help control the pipeline.

For example:

txt If red-team risk is high → require stronger caveats. If consensus is weak → reduce confidence. If coverage is low → regenerate missing sections. If judge score is poor → run another revision pass. 

---

## Internal Control Logic

A strong version of Berlin42 should not just run models in a fixed sequence.

It should make decisions.

Examples:

txt If candidates strongly disagree:     run extra judge pass  If factual confidence is low:     mark claims as uncertain  If red-team finds a major flaw:     revise before final answer  If answer is too generic:     force practical specificity  If reusable prompt is weak:     regenerate prompt with stricter structure  If judge model rejects final answer:     return to consensus stage 

This makes the system closer to an answer engine than a prompt chain.

---

## System Architecture

txt Frontend   ↓ Backend API   ↓ Payment Gate   ↓ Run Orchestrator   ↓ Model Router   ↓ Pipeline Stages       - task brief       - candidate generation       - critique       - red-team       - judge       - consensus       - final formatter       - reusable prompt extractor   ↓ Run State   ↓ Frontend Polling 

The backend owns the orchestration logic.

The frontend is only a control and display layer.

---

## Running Locally

### Backend

bash cd backend cargo run 

Default backend:

txt http://127.0.0.1:8080 

### Frontend

bash cd frontend npm install npm run dev 

Default frontend:

txt http://127.0.0.1:5173 

---

## Useful API Endpoints

### Health

bash curl http://127.0.0.1:8080/health 

### Start Run

bash curl -sS http://127.0.0.1:8080/api/runs \   -H 'Content-Type: application/json' \   -d '{     "intent": "Analyze whether this company is trustworthy",     "artifact_type": "answer",     "variant_count": 1,     "research_enabled": false,     "execution_mode": "live"   }' 

### Get Run

bash curl -sS http://127.0.0.1:8080/api/runs/<run_id> 

---

## Environment

Example environment variables:

env GOOGLE_CLOUD_PROJECT= GOOGLE_APPLICATION_CREDENTIALS_JSON=  X402_ENABLED=false X402_PAY_TO= X402_FACILITATOR_URL=https://facilitator.goplausible.xyz 

Do not commit secrets, private keys, mnemonics, service-account files, or local .env files.

---

## Repository Structure

txt . ├── backend/              # Rust API server and orchestration logic ├── frontend/             # React/Vite interface ├── docs/                 # Architecture and pipeline notes ├── evals/                # Evaluation examples and scoring ideas ├── infra/                # Deployment-related files ├── prompts/              # Prompt templates and role instructions ├── scripts/              # Smoke tests and helper scripts ├── skills/               # Reusable prompt/skill artifacts ├── config_models.json    # Model routing configuration ├── Dockerfile └── README.md 

---

## One-Line Summary

Berlin42 Answer Forge is a multi-agent answer engine that improves LLM output through critique, red-team review, judge-model scoring, consensus merging, and reusable prompt extraction.