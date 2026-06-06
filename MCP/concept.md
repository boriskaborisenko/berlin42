# NestorChat MCP: paid consensus answers for agents

NestorChat can be used by humans through the web UI, but the same x402 gate is also interesting for AI agents.

The MCP idea is to expose NestorChat as a local tool that an agent can call when a normal one-shot model answer is not enough. The MCP server handles the API request, payment handshake, polling, and final formatting, then returns the answer and reusable prompt back to the agent.

## Why MCP matters

Human users can approve a wallet transaction in Defly or Pera. Autonomous agents cannot reliably stop every tool call and ask a human to sign one payment.

Without an MCP/payment adapter, the flow is awkward:

```text
Agent -> NestorChat API -> 402 Payment Required -> human wallet approval needed
```

That breaks the point of agentic workflows.

With MCP, payment can be handled by a local, explicit payment policy:

```text
Agent
-> local NestorChat MCP tool
-> x402 payment adapter
-> NestorChat API
-> answer + reusable prompt
```

## Tool shape

The first useful MCP tool is intentionally small:

```text
get_premium_answer(
  question: string,
  mode?: "live" | "mock",
  max_price_atomic?: string
)
```

It returns:

- original user question;
- polished NestorChat answer;
- reusable prompt;
- quality metrics when available;
- payment receipt / settlement metadata when available.

## Payment modes

There are several possible trust models. They should be explicit instead of hidden behind a magic "pay" button.

### 1. Session wallet

Best for autonomous agents and hackathon demo.

The MCP server is configured with a small, pre-funded Algorand TestNet session wallet. It signs x402 payment transactions locally and retries the request automatically after a `402 Payment Required`.

This is the same idea as the frontend demo session wallet, but safer operationally because it lives in a local MCP process rather than inside browser `localStorage`.

Requirements:

- TestNet ALGO for fees/minimum balance;
- opt-in to USDC ASA `10458941`;
- TestNet USDC balance;
- strict max-price guard before signing.

Risk:

- The session wallet is a hot wallet.
- It should contain only a small budget.
- It should not be a user's primary wallet.

### 2. Wallet-mediated mode

Best for human-in-the-loop usage.

The MCP server can request a wallet signature through a wallet connector, but the human still approves each transaction in Defly/Pera. This keeps custody clean, but it is not fully autonomous.

This mode is useful when the user wants explicit approval for every paid answer.

### 3. Dev bypass

Best for local development only.

The MCP server sends `X402-DEV-PAYMENT: dev-paid` when the backend has `X402_DEV_BYPASS=true`. No on-chain payment is made.

## x402 flow

```text
1. Agent calls get_premium_answer.
2. MCP server sends POST /api/runs.
3. Backend returns 402 with PAYMENT-REQUIRED.
4. MCP payment adapter checks price, network, asset, recipient, and timeout.
5. Adapter signs the payment payload.
6. MCP server retries with PAYMENT-SIGNATURE.
7. Backend verifies and settles through the facilitator.
8. Backend starts the NestorChat pipeline.
9. MCP server polls /api/runs/:id.
10. Agent receives answer, reusable prompt, metrics, and payment metadata.
```

## Guardrails

The MCP server should refuse payment when:

- network is not Algorand TestNet in demo mode;
- asset is not USDC ASA `10458941`;
- amount is above configured max spend;
- recipient does not match the configured NestorChat receiver;
- payment requirements are malformed;
- wallet balance is insufficient;
- backend tries to charge twice for the same tool call.

## Why this is valuable

NestorChat turns high-quality answer generation into a paid API primitive. MCP turns that primitive into something agents can actually use.

The product story becomes:

```text
Agents can buy better reasoning when the task deserves it.
```

That is stronger than just "a chatbot with payments". It is a small economic layer for agent workflows: request, pay, verify, receive answer, reuse prompt.

## Open design choices

The current best path is:

1. keep browser Session Wallet for demo speed;
2. add Defly/Pera wallet mode for human-approved web payments;
3. build MCP with a local session wallet and strict spend limits;
4. later explore WalletConnect in MCP if a good human-approval flow is needed.

Seed phrases are acceptable for a bounded TestNet/demo session wallet, but they should not be positioned as the ideal end-user experience. The clean product split is:

- humans use Defly/Pera;
- autonomous agents use a limited session wallet;
- developers use dev bypass while building.
