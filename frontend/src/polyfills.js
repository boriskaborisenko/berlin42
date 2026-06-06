import { Buffer } from "buffer";

globalThis.global = globalThis;
globalThis.Buffer = globalThis.Buffer || Buffer;
globalThis.process = globalThis.process || { env: {} };
