import React, { useEffect, useMemo, useState } from "react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import {
  AlertCircle,
  CheckCircle2,
  Clock3,
  Copy,
  Trash2,
  Loader2,
  LockKeyhole,
  Gauge,
  Send,
} from "lucide-react";

const API_BASE = import.meta.env.VITE_API_BASE_URL || "http://127.0.0.1:8080";
const STORAGE_KEY = "nestorchat.runs.v1";
const ACTIVE_RUN_KEY = "nestorchat.activeRunId.v1";
const DEV_PAYMENT_KEY = "nestorchat.devPayment.v1";
const wait = (ms) => new Promise((resolve) => window.setTimeout(resolve, ms));

export function App() {
  const [intent, setIntent] = useState("");
  const [runs, setRuns] = useState(readStoredRuns);
  const [activeRunId, setActiveRunId] = useState(readStoredActiveRunId);
  const [error, setError] = useState("");
  const [isRunning, setIsRunning] = useState(false);
  const [paymentRequired, setPaymentRequired] = useState(null);
  const [useDevPayment, setUseDevPayment] = useState(readStoredDevPayment);
  const [isComposing, setIsComposing] = useState(!readStoredActiveRunId());
  const [benchmarkingRunIds, setBenchmarkingRunIds] = useState([]);

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
    window.localStorage.setItem(DEV_PAYMENT_KEY, useDevPayment ? "true" : "false");
  }, [useDevPayment]);

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
      if (useDevPayment) {
        headers["X402-DEV-PAYMENT"] = "dev-paid";
      }

      const response = await fetch(`${API_BASE}/api/runs`, {
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
                  <button className="welcomeSendButton" type="submit" disabled={isRunning || !intent.trim()}>
                    {isRunning ? <Loader2 className="spin" size={18} /> : <Send size={18} />}
                  </button>
                </div>
                <label className="welcomeDevToggle">
                  <input
                    type="checkbox"
                    checked={useDevPayment}
                    onChange={(event) => setUseDevPayment(event.target.checked)}
                  />
                  <span>Use dev payment bypass</span>
                </label>
              </form>
              {error && !activeRun && (
                <div className="errorBox standaloneError" role="alert">
                  <AlertCircle size={18} />
                  <span>{error}</span>
                </div>
              )}
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
              {requirement.maxAmountRequired} atomic units (0.001 USDC)
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
