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
  WalletCards,
} from "lucide-react";

const API_BASE = import.meta.env.VITE_API_BASE_URL || "http://127.0.0.1:8080";
const STORAGE_KEY = "berlin42.answerForge.runs.v1";
const ACTIVE_RUN_KEY = "berlin42.answerForge.activeRunId.v1";
const wait = (ms) => new Promise((resolve) => window.setTimeout(resolve, ms));

export function App() {
  const [intent, setIntent] = useState("");
  const [runs, setRuns] = useState(readStoredRuns);
  const [activeRunId, setActiveRunId] = useState(readStoredActiveRunId);
  const [error, setError] = useState("");
  const [isRunning, setIsRunning] = useState(false);
  const [paymentRequired, setPaymentRequired] = useState(null);
  const [useDevPayment, setUseDevPayment] = useState(false);
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
        setPaymentRequired(data);
        throw new Error("Payment required before the live answer can start.");
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
            <p>Berlin42</p>
            <h1>Answer Forge</h1>
          </div>
          <span className="liveDot" aria-label="Backend status" />
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
          <label className="devToggle">
            <input
              type="checkbox"
              checked={useDevPayment}
              onChange={(event) => setUseDevPayment(event.target.checked)}
            />
            <span>Dev payment</span>
          </label>
        </header>

        <section className={`answerPane ${isComposing || isRunning ? "answerPaneWithComposer" : ""}`} aria-live="polite">
          {!activeRun && !isRunning && !paymentRequired && (
            <div className="welcome">
              <WalletCards size={34} />
              <h2>Ask once. Pay once. Get a stronger answer.</h2>
              <p>
                The backend gates live runs with x402, then routes your request through the
                multi-model pipeline.
              </p>
            </div>
          )}

          {paymentRequired && (
            <PaymentNotice paymentRequired={paymentRequired} />
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
              <summary>Pipeline details</summary>
              {activeRun.stages.map((stage, index) => (
                <div className="stageRow" key={`${stage.stage}-${stage.model_id}-${index}`}>
                  <span>{stage.stage}</span>
                  <small>{stage.model_id}</small>
                </div>
              ))}
            </details>
          )}

          {error && (
            <div className="errorBox" role="alert">
              <AlertCircle size={18} />
              <span>{error}</span>
            </div>
          )}
        </section>

        {(isComposing || isRunning) && (
          <form className="promptBar" onSubmit={startRun}>
            <div className="promptBox">
              <label className="promptInput">
                <span className="srOnly">Request</span>
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
              </label>
              <button className="sendButton" type="submit" disabled={isRunning || !intent.trim()}>
                {isRunning ? <Loader2 className="spin" size={18} /> : <Send size={18} />}
              </button>
            </div>
          </form>
        )}
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

  return (
    <section className="qualityCard" aria-label="Quality metrics">
      <div className="qualityCardHeader">
        <Gauge size={19} />
        <div>
          <h3>Quality analysis</h3>
          <p>{metrics.summary || "Calculated from the model drafts behind this answer."}</p>
        </div>
      </div>

      <div className="metricGrid">
        <MetricTile
          label={metrics.fci?.label || "Consensus confidence"}
          value={`${metrics.fci?.score ?? 0}%`}
          annotation={
            metrics.fci?.annotation ||
            "Checks how many key claims were supported by at least two independent model drafts."
          }
        />
        <MetricTile
          label={metrics.acg?.label || "Coverage lift"}
          value={`+${Math.max(metrics.acg?.coverage_gain ?? 0, 0)}%`}
          annotation={
            metrics.acg?.annotation ||
            "Compares final answer coverage against the best single model draft."
          }
        />
      </div>

      {run.benchmark_status === "error" && (
        <p className="qualityWarning">Benchmark model failed; fallback metrics are shown.</p>
      )}

      {metrics.notes?.length > 0 && (
        <details className="qualityNotes">
          <summary>Metric notes</summary>
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

function MetricTile({ label, value, annotation }) {
  return (
    <article className="metricTile">
      <span>{label}</span>
      <strong>{value}</strong>
      <p>{annotation}</p>
    </article>
  );
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

function PaymentNotice({ paymentRequired }) {
  const requirement = paymentRequired.accepts?.[0];

  return (
    <article className="paymentNotice">
      <LockKeyhole size={24} />
      <div>
        <h2>Payment required</h2>
        <p>Live answers are protected by x402. Connect the wallet flow, then retry with a payment signature.</p>
        {requirement && (
          <dl>
            <div>
              <dt>Network</dt>
              <dd>{requirement.network}</dd>
            </div>
            <div>
              <dt>Asset</dt>
              <dd>{requirement.asset}</dd>
            </div>
            <div>
              <dt>Amount</dt>
              <dd>{requirement.maxAmountRequired}</dd>
            </div>
          </dl>
        )}
      </div>
    </article>
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
