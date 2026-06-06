# NestorChat Frontend

React/Vite interface for the NestorChat demo.

For the product overview, architecture, payment flow, and backend setup, read the root [README.md](../README.md).

## Development

Install dependencies:

```sh
npm install
```

Run the local frontend:

```sh
npm run dev
```

Build:

```sh
npm run build
```

The app expects the backend at `http://127.0.0.1:8080` unless `VITE_API_BASE_URL` is set.

## What This UI Shows

- request input with optional dev payment bypass;
- local history stored in `localStorage`;
- markdown final answer;
- reusable prompt;
- async quality card comparing `Gemini 3.5 Flash only` vs `NestorChat consensus`;
- payment-required modal for real x402 flow.
