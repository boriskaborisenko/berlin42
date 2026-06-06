import { Buffer } from "node:buffer";
import { config } from "dotenv";
import { x402Client, wrapFetchWithPayment, x402HTTPClient } from "@x402/fetch";
import { toClientAvmSigner, ExactAvmScheme, ALGORAND_TESTNET_CAIP2 } from "@x402/avm";
import { ed25519SigningKeyFromWrappedSecret } from "@algorandfoundation/algokit-utils/crypto";
import { seedFromMnemonic } from "@algorandfoundation/algokit-utils/algo25";

config({ path: "../.env" });

const apiBase = process.env.VITE_API_BASE_URL || "http://localhost:8080";
const avmMnemonic = process.env.AVM_MNEMONIC;

if (!avmMnemonic || avmMnemonic.startsWith("TODO_")) {
  console.error("Missing AVM_MNEMONIC in root .env");
  process.exit(1);
}

const intent = process.argv.slice(2).join(" ").trim() || "какой самый умный кот?";

async function main() {
  const secretKey = await getSecretKeyFromMnemonic(avmMnemonic);
  const avmSigner = toClientAvmSigner(secretKey);
  const client = new x402Client();
  client.register(ALGORAND_TESTNET_CAIP2, new ExactAvmScheme(avmSigner));

  const fetchWithPayment = wrapFetchWithPayment(fetch, client);
  const response = await fetchWithPayment(`${apiBase}/api/runs`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      intent,
      artifact_type: "answer",
      variant_count: 1,
      research_enabled: false,
      execution_mode: "live",
    }),
  });

  const paymentResponse = new x402HTTPClient(client).getPaymentSettleResponse((name) =>
    response.headers.get(name),
  );
  const body = await response.json();

  console.log("payer:", avmSigner.address);
  console.log("status:", response.status, response.statusText);
  console.log("payment:", JSON.stringify(paymentResponse, null, 2));
  console.log("body:", JSON.stringify(body, null, 2));
}

async function getSecretKeyFromMnemonic(mnemonic) {
  const seed = seedFromMnemonic(mnemonic);
  const seedCopy = new Uint8Array(seed);
  const wrappedSeed = {
    unwrapEd25519Seed: async () => seed,
    wrapEd25519Seed: async () => {},
  };
  const wrappedSecret = await ed25519SigningKeyFromWrappedSecret(wrappedSeed);
  return Buffer.concat([Buffer.from(seedCopy), Buffer.from(wrappedSecret.ed25519Pubkey)]).toString(
    "base64",
  );
}

main().catch((error) => {
  console.error(error?.response?.data?.error ?? error);
  process.exit(1);
});
