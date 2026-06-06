# TriForge Frontend

React/Vite app for running and inspecting TriForge pipeline runs.

## Planned Responsibilities

- collect the user's raw intent
- choose artifact mode: skill, prompt, or both
- toggle research
- show pipeline stages and model outputs
- display final artifact with copy/export controls
- support revision feedback

## Development

The frontend will run separately during development, likely with Vite:

```sh
npm run dev
```

The Rust backend remains the source of truth for pipeline execution.
