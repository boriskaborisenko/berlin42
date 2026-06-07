# NestorChat MCP Scaffold

This folder is a draft MCP integration for NestorChat.

It is intentionally a scaffold, not a finished production server. The goal is to show how an agent could call NestorChat as a paid premium-answer tool while keeping payment policy explicit.

## Tool

```text
nestor_get_premium_answer
```

Input:

```json
{
  "question": "What is the smartest cat?",
  "mode": "live",
  "max_price_atomic": "1000",
  "include_quality_metrics": true
}
```

Output:

- original question;
- final NestorChat answer;
- reusable prompt;
- quality metrics, if available;
- payment metadata, if available.

## Intended Payment Modes

### dev

Sends `X402-DEV-PAYMENT: dev-paid`.

Useful while building the MCP surface locally.

### session

Uses a small pre-funded Algorand TestNet hot wallet controlled by the MCP process.

This is the most realistic autonomous-agent mode, but it must have strict spend limits.

### wallet

Reserved for later. A human approves payment in Defly/Pera through WalletConnect.

This is better custody, but weaker automation.

## Files

```text
concept.md                 Product concept and trust model
package.json               Draft Node/TypeScript package metadata
tsconfig.json              Draft TypeScript config
.env.example               MCP-specific local config
examples/tool-call.json    Example MCP tool payload
src/server.ts              MCP server entrypoint skeleton
src/nestor-client.ts       NestorChat API client skeleton
src/payment-policy.ts      x402 guardrail checks
src/types.ts               Shared types
```

## Development Direction

1. Start with `payment_mode=dev`.
2. Make the MCP tool call the local backend and poll until completion.
3. Add session-wallet x402 signing with strict max spend checks.
4. Add payment receipt handling.
5. Only then consider WalletConnect/human approval.

## Important

Do not put a real seed phrase or funded mainnet wallet into this scaffold.

For demos, use Algorand TestNet only and keep session-wallet funds tiny.
