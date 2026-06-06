import "./polyfills.js";
import React from "react";
import { createRoot } from "react-dom/client";
import { NetworkId, WalletId, WalletManager, WalletProvider } from "@txnlab/use-wallet-react";
import { App } from "./App.jsx";
import "./styles.css";

const walletManager = new WalletManager({
  defaultNetwork: NetworkId.TESTNET,
  wallets: [
    { id: WalletId.DEFLY, options: { chainId: 416002 } },
    { id: WalletId.PERA, options: { chainId: 416002 } },
  ],
});

createRoot(document.getElementById("root")).render(
  <React.StrictMode>
    <WalletProvider manager={walletManager}>
      <App />
    </WalletProvider>
  </React.StrictMode>,
);
