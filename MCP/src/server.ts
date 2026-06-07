import { Server } from "@modelcontextprotocol/sdk/server/index.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import {
  CallToolRequestSchema,
  ListToolsRequestSchema,
} from "@modelcontextprotocol/sdk/types.js";
import { z } from "zod";
import { getPremiumAnswer } from "./nestor-client.js";
import { readConfig } from "./payment-policy.js";

const PremiumAnswerSchema = z.object({
  question: z.string().min(1),
  mode: z.enum(["live", "mock"]).optional(),
  max_price_atomic: z.string().optional(),
  include_quality_metrics: z.boolean().optional(),
});

const server = new Server(
  {
    name: "nestorchat-mcp",
    version: "0.1.0",
  },
  {
    capabilities: {
      tools: {},
    },
  },
);

server.setRequestHandler(ListToolsRequestSchema, async () => ({
  tools: [
    {
      name: "nestor_get_premium_answer",
      description:
        "Ask NestorChat for a paid consensus answer and reusable prompt. Scaffolded for dev mode first.",
      inputSchema: {
        type: "object",
        required: ["question"],
        properties: {
          question: {
            type: "string",
            description: "User question or task.",
          },
          mode: {
            type: "string",
            enum: ["live", "mock"],
            description: "NestorChat execution mode.",
          },
          max_price_atomic: {
            type: "string",
            description: "Maximum allowed x402 price in atomic USDC units.",
          },
          include_quality_metrics: {
            type: "boolean",
            description: "Whether to include benchmark metrics in the tool result.",
          },
        },
      },
    },
  ],
}));

server.setRequestHandler(CallToolRequestSchema, async (request) => {
  if (request.params.name !== "nestor_get_premium_answer") {
    throw new Error(`Unknown tool: ${request.params.name}`);
  }

  const input = PremiumAnswerSchema.parse(request.params.arguments || {});
  const result = await getPremiumAnswer(input, readConfig());

  return {
    content: [
      {
        type: "text",
        text: formatResult(result),
      },
    ],
  };
});

const transport = new StdioServerTransport();
await server.connect(transport);

function formatResult(result: Awaited<ReturnType<typeof getPremiumAnswer>>) {
  return [
    `# NestorChat Answer`,
    ``,
    `## Question`,
    result.question,
    ``,
    `## Answer`,
    result.answer || "_No answer returned._",
    ``,
    `## Reusable Prompt`,
    result.reusable_prompt || "_No reusable prompt returned._",
    ``,
    `## Metadata`,
    "```json",
    JSON.stringify(
      {
        run_id: result.run_id,
        status: result.status,
        payment: result.payment,
        quality_metrics: result.quality_metrics,
      },
      null,
      2,
    ),
    "```",
  ].join("\n");
}
