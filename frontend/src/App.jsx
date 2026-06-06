import React, { useEffect, useMemo, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Buffer } from "buffer";
import { x402Client, wrapFetchWithPayment } from "@x402/fetch";
import { ExactAvmScheme } from "@x402/avm/exact/client";
import { mnemonicFromSeed, seedFromMnemonic } from "@algorandfoundation/algokit-utils/algo25";
import { decodeTransaction, generateAddressWithSigners } from "@algorandfoundation/algokit-utils/transact";
import * as ed25519 from "@noble/ed25519";
import { sha512 } from "@noble/hashes/sha2.js";
import {
  AlertCircle,
  CheckCircle2,
  Clock3,
  Copy,
  ExternalLink,
  Trash2,
  Loader2,
  LockKeyhole,
  Gauge,
  KeyRound,
  Send,
  ShieldCheck,
  Wallet,
} from "lucide-react";

const API_BASE = import.meta.env.VITE_API_BASE_URL || "http://127.0.0.1:8080";
const STORAGE_KEY = "nestorchat.runs.v1";
const ACTIVE_RUN_KEY = "nestorchat.activeRunId.v1";
const DEV_PAYMENT_KEY = "nestorchat.devPayment.v1";
const PAYMENT_MODE_KEY = "nestorchat.paymentMode.v1";
const SESSION_WALLET_KEY = "nestorchat.sessionWallet.v1";
const ALGORAND_TESTNET_CAIP2 = "algorand:SGO1GKSzyE7IEPItTxCByw9x8FmnrCDexi9/cOUJOiI=";
const wait = (ms) => new Promise((resolve) => window.setTimeout(resolve, ms));

if (typeof window !== "undefined" && !window.Buffer) {
  window.Buffer = Buffer;
}
ed25519.hashes.sha512 = sha512;

export function App() {
  const [intent, setIntent] = useState("");
  const [runs, setRuns] = useState(readStoredRuns);
  const [activeRunId, setActiveRunId] = useState(readStoredActiveRunId);
  const [error, setError] = useState("");
  const [isRunning, setIsRunning] = useState(false);
  const [paymentRequired, setPaymentRequired] = useState(null);
  const [paymentMode, setPaymentMode] = useState(readStoredPaymentMode);
  const [sessionWallet, setSessionWallet] = useState(readStoredSessionWallet);
  const [sessionMnemonic, setSessionMnemonic] = useState("");
  const [walletError, setWalletError] = useState("");
  const [isImportingWallet, setIsImportingWallet] = useState(false);
  const [isComposing, setIsComposing] = useState(!readStoredActiveRunId());
  const [benchmarkingRunIds, setBenchmarkingRunIds] = useState([]);
  const useDevPayment = paymentMode === "dev";
  const isSessionPaymentReady = paymentMode !== "session" || Boolean(sessionWallet?.secretKey);

  const activeRun = useMemo(
    () => runs.find((run) => run.run_id === activeRunId) || null,
    [runs, activeRunId],
  );

  useEffect(() => {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(runs));
  }, [runs]);

  useEffect(() => {
    if (activeRunId) {
      window.localStorage.setItem(ACTIVE_RUN_KEY, activeRunId);
    } else {
      window.localStorage.removeItem(ACTIVE_RUN_KEY);
    }
  }, [activeRunId]);

  useEffect(() => {
    window.localStorage.setItem(PAYMENT_MODE_KEY, paymentMode);
    window.localStorage.setItem(DEV_PAYMENT_KEY, useDevPayment ? "true" : "false");
  }, [paymentMode, useDevPayment]);

  useEffect(() => {
    if (sessionWallet) {
      window.localStorage.setItem(SESSION_WALLET_KEY, JSON.stringify(sessionWallet));
    } else {
      window.localStorage.removeItem(SESSION_WALLET_KEY);
    }
  }, [sessionWallet]);

  useEffect(() => {
    if (activeRunId && !runs.some((run) => run.run_id === activeRunId)) {
      setActiveRunId(runs[0]?.run_id || null);
    }
  }, [activeRunId, runs]);

  function upsertRun(nextRun) {
    setRuns((currentRuns) => {
      const exists = currentRuns.some((run) => run.run_id === nextRun.run_id);
      if (exists) {
        return currentRuns.map((run) => (run.run_id === nextRun.run_id ? nextRun : run));
      }
      return [nextRun, ...currentRuns];
    });
    setActiveRunId(nextRun.run_id);
    setIsComposing(false);
  }

  function beginNewRequest() {
    setActiveRunId(null);
    setPaymentRequired(null);
    setError("");
    setIsComposing(true);
  }

  function deleteRun(runId) {
    setRuns((currentRuns) => currentRuns.filter((run) => run.run_id !== runId));
    setActiveRunId((currentActiveRunId) => (currentActiveRunId === runId ? null : currentActiveRunId));
    setBenchmarkingRunIds((runIds) => runIds.filter((id) => id !== runId));
  }

  function generateSessionWallet() {
    setWalletError("");

    try {
      const seed = window.crypto.getRandomValues(new Uint8Array(32));
      saveSessionWalletFromSecretKey(secretKeyFromSeed(seed), mnemonicFromSeed(seed));
      setSessionMnemonic("");
      setPaymentMode("session");
    } catch (walletGenerateError) {
      setWalletError(walletGenerateError.message || "Could not generate a session wallet.");
    }
  }

  async function importSessionWallet(event) {
    event.preventDefault();
    const mnemonic = sessionMnemonic.trim().replace(/\s+/g, " ");

    if (!mnemonic) {
      setWalletError("Paste the 25-word Algorand TestNet mnemonic first.");
      return;
    }

    setIsImportingWallet(true);
    setWalletError("");

    try {
      saveSessionWalletFromSecretKey(await getSecretKeyFromMnemonic(mnemonic), mnemonic);
      setSessionMnemonic("");
      setPaymentMode("session");
    } catch (walletImportError) {
      setWalletError(walletImportError.message || "Could not import this mnemonic.");
    } finally {
      setIsImportingWallet(false);
    }
  }

  function saveSessionWalletFromSecretKey(secretKey, mnemonic) {
    const signer = toBrowserAvmSigner(secretKey);
    setSessionWallet({
      address: signer.address,
      secretKey,
      mnemonic,
      imported_at: new Date().toISOString(),
    });
  }

  function clearSessionWallet() {
    setSessionWallet(null);
    setSessionMnemonic("");
    setWalletError("");
  }

  async function pollRun(runId) {
    for (;;) {
      await wait(1000);
      const response = await fetch(`${API_BASE}/api/runs/${runId}`);
      const data = await response.json();

      if (!response.ok || data.error) {
        throw new Error(data.error || `Polling failed with ${response.status}`);
      }

      upsertRun(data);

      if (data.status === "completed") return data;
      if (data.status === "error") throw new Error(data.error || "Run failed");
    }
  }

  async function pollBenchmark(runId) {
    setBenchmarkingRunIds((runIds) => (runIds.includes(runId) ? runIds : [...runIds, runId]));

    try {
      for (;;) {
        await wait(1500);
        const response = await fetch(`${API_BASE}/api/runs/${runId}`);
        const data = await response.json();

        if (!response.ok || data.error) {
          throw new Error(data.error || `Benchmark polling failed with ${response.status}`);
        }

        upsertRun(data);

        if (!isBenchmarkPending(data)) return data;
      }
    } finally {
      setBenchmarkingRunIds((runIds) => runIds.filter((id) => id !== runId));
    }
  }

  async function startRun(event) {
    event.preventDefault();

    const trimmedIntent = intent.trim();
    if (!trimmedIntent || isRunning) return;

    setIsRunning(true);
    setError("");
    setPaymentRequired(null);

    try {
      const headers = { "Content-Type": "application/json" };
      let requestFetch = fetch;

      if (useDevPayment) {
        headers["X402-DEV-PAYMENT"] = "dev-paid";
      } else if (paymentMode === "session") {
        if (!sessionWallet?.secretKey) {
          throw new Error("Import a funded Algorand TestNet session wallet before sending a paid request.");
        }

        const avmSigner = toBrowserAvmSigner(sessionWallet.secretKey);
        const client = new x402Client();
        client.register(ALGORAND_TESTNET_CAIP2, new ExactAvmScheme(avmSigner));
        requestFetch = wrapFetchWithPayment(fetch, client);
      }

      const response = await requestFetch(`${API_BASE}/api/runs`, {
        method: "POST",
        headers,
        body: JSON.stringify({
          intent: trimmedIntent,
          artifact_type: "answer",
          variant_count: 1,
          research_enabled: false,
          execution_mode: "live",
        }),
      });

      const data = await response.json();

      if (response.status === 402) {
        if (useDevPayment) {
          throw new Error(
            "Dev payment is enabled, but the backend still returned 402. Restart the backend and check X402_DEV_BYPASS=true plus X402_DEV_BYPASS_TOKEN=dev-paid in the root .env.",
          );
        }
        setPaymentRequired(data);
        return;
      }

      if (!response.ok || data.error) {
        throw new Error(data.error || `Request failed with ${response.status}`);
      }

      upsertRun(data);
      setIntent("");

      if (data.status === "running") {
        const completedRun = await pollRun(data.run_id);
        if (isBenchmarkPending(completedRun)) {
          pollBenchmark(completedRun.run_id).catch((benchmarkError) => {
            setError(benchmarkError.message);
          });
        }
      }
    } catch (runError) {
      setError(runError.message);
    } finally {
      setIsRunning(false);
    }
  }

  const answer = activeRun?.final_variants?.[0]?.content || "";
  const reusablePrompt = activeRun?.final_variants?.[0]?.reusable_prompt || "";
  const progressLabel = getProgressLabel(activeRun, isRunning);

  return (
    <main className="appShell">
      <aside className="sidebar" aria-label="Request history">
        <div className="sidebarHeader">
          <div>
            <h1>NestorChat</h1>
          </div>
        </div>

        <button className="newButton" type="button" onClick={beginNewRequest}>
          New request
        </button>

        <nav className="historyList" aria-label="Previous requests">
          {runs.length === 0 && <p className="emptyHistory">Your paid answers will appear here.</p>}
          {runs.map((run) => (
            <div
              className={`historyItem ${run.run_id === activeRunId ? "historyItemActive" : ""}`}
              key={run.run_id}
            >
              <button className="historySelect" type="button" onClick={() => setActiveRunId(run.run_id)}>
                <span>{run.intent || run.final_variants?.[0]?.title || "Running answer"}</span>
                <small>{run.status}</small>
              </button>
              <button
                className="deleteButton"
                type="button"
                aria-label="Delete request"
                onClick={() => deleteRun(run.run_id)}
              >
                <Trash2 size={15} />
              </button>
            </div>
          ))}
        </nav>
      </aside>

      <section className="conversation" aria-label="Answer workspace">
        <header className="topbar">
          <StatusPill result={activeRun} isRunning={isRunning} label={progressLabel} />
        </header>

        <section className="answerPane" aria-live="polite">
          {!activeRun && !isRunning && (
            <div className="welcome">
              <div className="welcomePaymentBadge">
                Algorand Testnet • 0.001 USDC (ASA 10458941) • Gated by x402
              </div>
              <h2>Ask once. Pay once. Get a stronger answer.</h2>
              <p>
                The backend gates live runs with x402, then routes your request through the
                multi-model pipeline.
              </p>

              <form className="welcomeForm" onSubmit={startRun}>
                <div className="welcomeInputContainer">
                  <textarea
                    value={intent}
                    onChange={(event) => setIntent(event.target.value)}
                    placeholder="Ask anything worth a premium answer..."
                    rows={1}
                    onKeyDown={(event) => {
                      if (event.key === "Enter" && !event.shiftKey) {
                        event.preventDefault();
                        startRun(event);
                      }
                    }}
                  />
                  <button
                    className="welcomeSendButton"
                    type="submit"
                    disabled={isRunning || !intent.trim() || !isSessionPaymentReady}
                  >
                    {isRunning ? <Loader2 className="spin" size={18} /> : <Send size={18} />}
                  </button>
                </div>
              </form>

              <PaymentMethodPanel
                paymentMode={paymentMode}
                setPaymentMode={setPaymentMode}
                sessionWallet={sessionWallet}
                sessionMnemonic={sessionMnemonic}
                setSessionMnemonic={setSessionMnemonic}
                walletError={walletError}
                isImportingWallet={isImportingWallet}
                onGenerateSessionWallet={generateSessionWallet}
                onImportSessionWallet={importSessionWallet}
                onClearSessionWallet={clearSessionWallet}
              />

              <p className="welcomeHowItWorks">
                Pipeline: Raw Query ➔ Base Brief ➔ 3 Parallel Candidates ➔ Real Cross-Review ➔ Distributed Red-Team ➔ Consensus Merge, Compression, Eval & Revision ➔ Verified answer + portable prompt with async quality benchmarks
              </p>
            </div>
          )}

          {paymentRequired && (
            <PaymentNoticeModal
              paymentRequired={paymentRequired}
              onClose={() => {
                setPaymentRequired(null);
              }}
            />
          )}

          {activeRun && (
            <article className="answerCard">
              <div className="answerHeader">
                <div>
                  <p>{activeRun.run_id}</p>
                  <h2>{activeRun.status === "completed" ? "Answer + prompt" : "Working on it"}</h2>
                </div>
                {answer && (
                  <button
                    className="iconButton"
                    type="button"
                    onClick={() => navigator.clipboard?.writeText(`${answer}\n\n---\n\n${reusablePrompt}`)}
                  >
                    <Copy size={17} />
                    <span>Copy all</span>
                  </button>
                )}
              </div>

              {answer ? (
                <div className="resultStack">
                  {activeRun.intent && (
                    <section className="requestSection" aria-label="Original request">
                      <span>Request</span>
                      <p>{activeRun.intent}</p>
                    </section>
                  )}

                  <QualityMetricsCard run={activeRun} isBenchmarking={benchmarkingRunIds.includes(activeRun.run_id)} />

                  <section className="resultSection">
                    <div className="sectionHeader">
                      <h3>Answer</h3>
                      <button className="textButton" type="button" onClick={() => navigator.clipboard?.writeText(answer)}>
                        Copy answer
                      </button>
                    </div>
                    <div className="markdownBody">
                      <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
                        {answer}
                      </ReactMarkdown>
                    </div>
                  </section>

                  {reusablePrompt && (
                    <section className="resultSection promptSection">
                      <div className="sectionHeader">
                        <h3>Reusable prompt</h3>
                        <button
                          className="textButton"
                          type="button"
                          onClick={() => navigator.clipboard?.writeText(reusablePrompt)}
                        >
                          Copy prompt
                        </button>
                      </div>
                      <div className="markdownBody promptBody">
                        <ReactMarkdown remarkPlugins={[remarkGfm]} components={markdownComponents}>
                          {reusablePrompt}
                        </ReactMarkdown>
                      </div>
                    </section>
                  )}
                </div>
              ) : (
                <div className="thinking">
                  <Loader2 className="spin" size={22} />
                  <span>{progressLabel}</span>
                </div>
              )}
            </article>
          )}

          {activeRun?.stages?.length > 0 && (
            <details className="stageDetails">
              <summary>
                <span>Pipeline details</span>
                {activeRun.run_duration_ms != null && (
                  <strong>{formatDuration(activeRun.run_duration_ms)} total</strong>
                )}
              </summary>
              {activeRun.stages.map((stage, index) => (
                <div className="stageRow" key={`${stage.stage}-${stage.model_id}-${index}`}>
                  <span>{stage.stage}</span>
                  <small>
                    <span>{stage.model_id}</span>
                    {stage.duration_ms != null && <strong>{formatDuration(stage.duration_ms)}</strong>}
                  </small>
                </div>
              ))}
            </details>
          )}

          {error && activeRun && (
            <div className="errorBox" role="alert">
              <AlertCircle size={18} />
              <span>{error}</span>
            </div>
          )}
        </section>

      </section>
    </main>
  );
}

function isBenchmarkPending(run) {
  return run?.benchmark_status === "pending" || run?.benchmark_status === "running";
}

function QualityMetricsCard({ run, isBenchmarking }) {
  if (!run?.final_variants?.length) return null;

  const metrics = run.quality_metrics;
  const pending = isBenchmarking || isBenchmarkPending(run);

  if (pending && !metrics) {
    return (
      <section className="qualityCard qualityCardLoading" aria-label="Quality metrics">
        <div className="qualityCardHeader">
          <Gauge size={19} />
          <div>
            <h3>Quality analysis</h3>
            <p>Calculated from the model drafts behind this answer.</p>
          </div>
        </div>
        <div className="qualityLoading">
          <Loader2 className="spin" size={18} />
          <span>Checking consensus confidence and coverage...</span>
        </div>
      </section>
    );
  }

  if (!metrics) {
    return null;
  }

  const fciScore = clampPercent(metrics.fci?.score ?? 0);
  const finalCoverage = clampPercent(metrics.acg?.final_coverage ?? 0);
  const bestSingleCoverage = clampPercent(metrics.acg?.best_single_model_coverage ?? 0);
  const coverageGain = finalCoverage - bestSingleCoverage;
  const verdict = getConsensusVerdict(fciScore, finalCoverage, coverageGain);
  const supportedClaims = metrics.fci?.supported_claims ?? 0;
  const totalClaims = metrics.fci?.total_claims ?? 0;
  const coveredAspects = metrics.acg?.covered_aspects ?? 0;
  const totalAspects = metrics.acg?.total_aspects ?? 0;
  const gainLabel = coverageGain >= 0 ? `+${coverageGain}%` : `${coverageGain}%`;
  const baselineLabel = "Gemini 3.5 Flash only";
  const consensusLabel = "NestorChat consensus";

  return (
    <section className="qualityCard" aria-label="Quality metrics">
      <div className="qualityCardHeader">
        <div className="qualityIcon">
          <Gauge size={18} />
        </div>
        <div className="qualityTitleGroup">
          <div className="qualityTitleLine">
            <h3>Quality check</h3>
            <span className={`qualityVerdict ${verdict.className}`}>{verdict.label}</span>
          </div>
          <p>
            {fciScore}% facts cross-confirmed • {gainLabel} completeness vs Gemini 3.5 Flash only
          </p>
        </div>
      </div>

      <div className="qualityProofGrid">
        <QualityProof
          label="Fact confidence"
          value={`${fciScore}%`}
          detail={
            totalClaims > 0
              ? `${supportedClaims}/${totalClaims} key claims were supported by multiple model drafts.`
              : "No key claims were available for cross-checking."
          }
        />
        <QualityProof
          label="Coverage lift"
          value={gainLabel}
          detail={
            totalAspects > 0
              ? `Final answer covers ${coveredAspects}/${totalAspects} required aspects.`
              : "Coverage was estimated from the available model drafts."
          }
        />
      </div>

      <div className="headToHeadCompare" aria-label="Head-to-head completeness comparison">
        <div className="headToHeadHeader">
          <span>Head-to-head completeness</span>
          <strong>{gainLabel}</strong>
        </div>
        <CoverageBar
          label={baselineLabel}
          value={bestSingleCoverage}
          note="Best single raw draft"
        />
        <CoverageBar
          label={consensusLabel}
          value={finalCoverage}
          note="Multi-draft synthesis"
          isPrimary
        />
      </div>

      {run.benchmark_status === "error" && (
        <p className="qualityWarning">Benchmark model failed; fallback metrics are shown.</p>
      )}

      <details className="qualityNotes">
        <summary>How this works</summary>
        <p>
          We extract key claims and answer aspects from the final answer and the raw model drafts.
          A claim counts as confident when at least two independent drafts support it.
        </p>
      </details>

      {metrics.notes?.length > 0 && (
        <details className="qualityNotes">
          <summary>Benchmark notes</summary>
          <ul>
            {metrics.notes.map((note, index) => (
              <li key={`${note}-${index}`}>{note}</li>
            ))}
          </ul>
        </details>
      )}
    </section>
  );
}

function QualityProof({ label, value, detail }) {
  return (
    <article className="qualityProof">
      <span>{label}</span>
      <strong>{value}</strong>
      <p>{detail}</p>
    </article>
  );
}

function CoverageBar({ label, value, note, isPrimary = false }) {
  const percent = clampPercent(value);

  return (
    <div className={`coverageBarRow ${isPrimary ? "coverageBarPrimary" : ""}`}>
      <div className="coverageBarLabel">
        <span>
          {label}
          {note && <small>{note}</small>}
        </span>
        <strong>{percent}%</strong>
      </div>
      <div className="coverageTrack" aria-hidden="true">
        <div className="coverageFill" style={{ width: `${percent}%` }} />
      </div>
    </div>
  );
}

function clampPercent(value) {
  const number = Number(value);
  if (!Number.isFinite(number)) return 0;
  return Math.max(0, Math.min(100, Math.round(number)));
}

function getConsensusVerdict(fciScore, finalCoverage, coverageGain) {
  if (fciScore >= 85 && finalCoverage >= 85 && coverageGain >= 10) {
    return { label: "Strong consensus", className: "qualityVerdictStrong" };
  }

  if (fciScore >= 70 && finalCoverage >= 70) {
    return { label: "Good consensus", className: "qualityVerdictGood" };
  }

  return { label: "Useful signal", className: "qualityVerdictNeutral" };
}

function readStoredRuns() {
  try {
    const raw = window.localStorage.getItem(STORAGE_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed : [];
  } catch {
    return [];
  }
}

function readStoredActiveRunId() {
  return window.localStorage.getItem(ACTIVE_RUN_KEY);
}

function readStoredDevPayment() {
  const stored = window.localStorage.getItem(DEV_PAYMENT_KEY);
  return stored === null ? true : stored === "true";
}

function readStoredPaymentMode() {
  const stored = window.localStorage.getItem(PAYMENT_MODE_KEY);
  if (stored === "dev" || stored === "session") return stored;
  return readStoredDevPayment() ? "dev" : "session";
}

function readStoredSessionWallet() {
  try {
    const raw = window.localStorage.getItem(SESSION_WALLET_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw);
    if (typeof parsed?.address !== "string" || typeof parsed?.secretKey !== "string") {
      return null;
    }
    toBrowserAvmSigner(parsed.secretKey);
    return parsed;
  } catch {
    return null;
  }
}

const markdownComponents = {
  a({ children, ...props }) {
    return (
      <a {...props} target="_blank" rel="noreferrer">
        {children}
      </a>
    );
  },
  code({ children, className, ...props }) {
    const isBlock = className?.startsWith("language-");
    if (isBlock) {
      return (
        <code className={className} {...props}>
          {children}
        </code>
      );
    }

    return (
      <code className="inlineCode" {...props}>
        {children}
      </code>
    );
  },
};

function PaymentMethodPanel({
  paymentMode,
  setPaymentMode,
  sessionWallet,
  sessionMnemonic,
  setSessionMnemonic,
  walletError,
  isImportingWallet,
  onGenerateSessionWallet,
  onImportSessionWallet,
  onClearSessionWallet,
}) {
  return (
    <section className="paymentPanel" aria-label="Payment method">
      <div className="paymentPanelHeader">
        <div>
          <span>Payment method</span>
          <strong>{paymentMode === "session" ? "Session wallet" : "Dev bypass"}</strong>
        </div>
        <div className="paymentSegment" role="tablist" aria-label="Payment mode">
          <button
            className={paymentMode === "session" ? "paymentSegmentActive" : ""}
            type="button"
            onClick={() => setPaymentMode("session")}
          >
            <Wallet size={15} />
            <span>Session</span>
          </button>
          <button
            className={paymentMode === "dev" ? "paymentSegmentActive" : ""}
            type="button"
            onClick={() => setPaymentMode("dev")}
          >
            <ShieldCheck size={15} />
            <span>Dev</span>
          </button>
        </div>
      </div>

      {paymentMode === "session" ? (
        <div className="sessionWalletBox">
          {sessionWallet ? (
            <div className="walletReady">
              <div className="walletReadyIcon">
                <KeyRound size={17} />
              </div>
              <div className="walletReadyBody">
                <span>Ready to auto-pay x402 requests</span>
                <code>{sessionWallet.address}</code>
              </div>
              <button
                className="walletGhostButton"
                type="button"
                onClick={() => navigator.clipboard?.writeText(sessionWallet.address)}
              >
                Copy
              </button>
              <button className="walletGhostButton danger" type="button" onClick={onClearSessionWallet}>
                Clear
              </button>
              {sessionWallet.mnemonic && (
                <div className="walletMnemonic">
                  <span>Demo recovery phrase</span>
                  <code>{sessionWallet.mnemonic}</code>
                  <button
                    className="walletGhostButton"
                    type="button"
                    onClick={() => navigator.clipboard?.writeText(sessionWallet.mnemonic)}
                  >
                    Copy phrase
                  </button>
                </div>
              )}
            </div>
          ) : (
            <div className="walletSetupStack">
              <button className="walletGenerateButton" type="button" onClick={onGenerateSessionWallet}>
                <Wallet size={16} />
                <span>Generate demo wallet</span>
              </button>
              <form className="walletImportForm" onSubmit={onImportSessionWallet}>
                <textarea
                  value={sessionMnemonic}
                  onChange={(event) => setSessionMnemonic(event.target.value)}
                  placeholder="Or paste a 25-word Algorand TestNet mnemonic. Stored locally for this demo wallet."
                  rows={3}
                />
                <button className="walletImportButton" type="submit" disabled={isImportingWallet}>
                  {isImportingWallet ? <Loader2 className="spin" size={16} /> : <KeyRound size={16} />}
                  <span>Import session wallet</span>
                </button>
              </form>
            </div>
          )}

          {walletError && (
            <div className="walletError" role="alert">
              <AlertCircle size={15} />
              <span>{walletError}</span>
            </div>
          )}

          <div className="walletFundingLinks">
            <a href="https://lora.algokit.io/testnet/fund" target="_blank" rel="noreferrer">
              ALGO faucet <ExternalLink size={13} />
            </a>
            <a href="https://faucet.circle.com/" target="_blank" rel="noreferrer">
              USDC faucet <ExternalLink size={13} />
            </a>
            <span>TestNet only. Fund ALGO, opt in to USDC ASA 10458941, then mint USDC.</span>
          </div>
        </div>
      ) : (
        <div className="devPaymentNote">
          <ShieldCheck size={16} />
          <span>Uses the backend dev bypass header. No on-chain payment is sent.</span>
        </div>
      )}
    </section>
  );
}

function StatusPill({ result, isRunning, label }) {
  if (isRunning || result?.status === "running") {
    return (
      <div className="statusPill statusActive">
        <Loader2 className="spin" size={16} />
        <span>{label}</span>
      </div>
    );
  }

  if (result?.status === "completed") {
    return (
      <div className="statusPill statusDone">
        <CheckCircle2 size={16} />
        <span>Completed</span>
      </div>
    );
  }

  return (
    <div className="statusPill">
      <Clock3 size={16} />
      <span>Ready</span>
    </div>
  );
}

async function getSecretKeyFromMnemonic(mnemonic) {
  const seed = seedFromMnemonic(mnemonic);
  return secretKeyFromSeed(seed);
}

function secretKeyFromSeed(seed) {
  const seedCopy = new Uint8Array(seed);
  const ed25519Pubkey = ed25519.getPublicKey(seed);
  return bytesToBase64(concatUint8(seedCopy, ed25519Pubkey));
}

function toBrowserAvmSigner(privateKeyBase64) {
  const secretKey = base64ToBytes(privateKeyBase64);
  if (secretKey.length !== 64) {
    throw new Error("AVM private key must be a Base64-encoded 64-byte key.");
  }

  const seed = secretKey.subarray(0, 32);
  const ed25519Pubkey = ed25519.getPublicKey(seed);
  const rawEd25519Signer = (bytesToSign) => ed25519.signAsync(bytesToSign, seed);
  const algokitSigners = generateAddressWithSigners({ ed25519Pubkey, rawEd25519Signer });

  return {
    address: algokitSigners.addr.toString(),
    signTransactions: async (txns, indexesToSign) => {
      return Promise.all(
        txns.map(async (txn, index) => {
          if (indexesToSign && !indexesToSign.includes(index)) return null;
          const decoded = decodeTransaction(txn);
          const signedBytes = await algokitSigners.signer([decoded], [0]);
          return signedBytes[0];
        }),
      );
    },
  };
}

function concatUint8(left, right) {
  const combined = new Uint8Array(left.length + right.length);
  combined.set(left, 0);
  combined.set(right, left.length);
  return combined;
}

function bytesToBase64(bytes) {
  let binary = "";
  const chunkSize = 0x8000;
  for (let index = 0; index < bytes.length; index += chunkSize) {
    binary += String.fromCharCode(...bytes.subarray(index, index + chunkSize));
  }
  return window.btoa(binary);
}

function base64ToBytes(base64) {
  const binary = window.atob(base64);
  const bytes = new Uint8Array(binary.length);
  for (let index = 0; index < binary.length; index += 1) {
    bytes[index] = binary.charCodeAt(index);
  }
  return bytes;
}

function PaymentNoticeModal({ paymentRequired, onClose }) {
  const requirement = paymentRequired?.accepts?.[0] || {
    network: "algorand:SGO1GKSzyE7IEPItTxCByw9x8FmnrCDexi9/cOUJOiI=",
    asset: "USDC",
    maxAmountRequired: "1000",
    assetId: "10458941",
  };

  return (
    <div className="modalOverlay" onClick={onClose}>
      <article className="modalContent" onClick={(event) => event.stopPropagation()}>
        <button className="modalCloseButton" type="button" onClick={onClose} aria-label="Close modal">
          ×
        </button>

        <div className="modalIcon">
          <LockKeyhole size={28} />
        </div>
        <h2>Payment Required</h2>
        <p>Live runs are protected by x402. Connect your wallet, sign the payment, and retry.</p>

        <div className="paymentDetailsTitle">Payment Requirements</div>
        <div className="paymentDetailsGrid">
          <div className="paymentDetailRow">
            <span className="paymentDetailLabel">Network</span>
            <strong className="paymentDetailValue">Algorand Testnet</strong>
          </div>
          <div className="paymentDetailRow">
            <span className="paymentDetailLabel">Asset</span>
            <strong className="paymentDetailValue">
              {requirement.asset} (ASA {requirement.assetId || "10458941"})
            </strong>
          </div>
          <div className="paymentDetailRow">
            <span className="paymentDetailLabel">Amount</span>
            <strong className="paymentDetailValue">
              {requirement.amount || requirement.maxAmountRequired} atomic units (0.001 USDC)
            </strong>
          </div>
        </div>

        <div className="modalActions">
          <button className="modalPrimaryButton" type="button" onClick={onClose}>
            Got it, let's retry
          </button>
        </div>
      </article>
    </div>
  );
}

function getProgressLabel(result, isRunning) {
  if (!isRunning && !result) return "Ready";
  if (result?.status === "completed") return "Completed";

  const stages = result?.stages || [];
  if (stages.some((stage) => stage.stage === "Final Formatting")) return "Finalizing answer";
  if (stages.some((stage) => stage.stage.includes("Merge"))) return "Merging model work";
  if (stages.filter((stage) => stage.stage === "Generation").length >= 1) return "Generating candidates";
  if (stages.some((stage) => stage.stage === "Base Prompt")) return "Building brief";
  return "Starting run";
}

function formatDuration(durationMs) {
  const ms = Number(durationMs);
  if (!Number.isFinite(ms) || ms < 0) return "0 ms";
  if (ms < 1000) return `${Math.round(ms)} ms`;

  const seconds = ms / 1000;
  if (seconds < 60) {
    return `${seconds.toFixed(seconds < 10 ? 1 : 0)}s`;
  }

  const minutes = Math.floor(seconds / 60);
  const remainingSeconds = Math.round(seconds % 60);
  return `${minutes}m ${remainingSeconds}s`;
}
