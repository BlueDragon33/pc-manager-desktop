import { useMemo, useState } from "react";

import {
  findingKindLabel,
  formatOptimizerPercent,
  runPerformanceOptimizer,
  sortPerformanceFindings,
  type PerformanceOptimizerReport,
} from "./performanceOptimizer";
import type { PageId } from "./navigation";

type OptimizerState =
  | { status: "idle" }
  | { status: "loading" }
  | { status: "ready"; report: PerformanceOptimizerReport }
  | { status: "error"; message: string };

function messageFromError(error: unknown): string {
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof (error as { message?: unknown }).message === "string"
  ) {
    return (error as { message: string }).message;
  }
  return error instanceof Error
    ? error.message
    : "Performance analysis could not complete safely.";
}

export function PerformanceOptimizerPage({
  onNavigate,
}: {
  onNavigate: (page: PageId) => void;
}) {
  const [state, setState] = useState<OptimizerState>({ status: "idle" });

  const runAnalysis = async () => {
    setState({ status: "loading" });
    try {
      const report = await runPerformanceOptimizer();
      setState({ status: "ready", report });
    } catch (error: unknown) {
      setState({ status: "error", message: messageFromError(error) });
    }
  };

  const findings = useMemo(
    () =>
      state.status === "ready"
        ? sortPerformanceFindings(state.report.findings)
        : [],
    [state],
  );

  return (
    <div className="page-stack p6-page">
      <section className="p6-hero">
        <div>
          <p className="eyebrow">Advanced Performance Optimizer — P12</p>
          <h2>Find repeated resource cost before changing Windows</h2>
          <p className="muted">
            PC Manager takes several short samples, then correlates repeatedly
            active processes with startup, scheduled-task, and service evidence.
            One spike is not enough to trigger a recommendation.
          </p>
        </div>
        <button
          className="primary-action"
          disabled={state.status === "loading"}
          onClick={() => void runAnalysis()}
          type="button"
        >
          {state.status === "loading"
            ? "Analyzing…"
            : "Run performance analysis"}
        </button>
      </section>

      <section className="info-callout">
        <strong>No “disable everything” mode exists.</strong>
        <span>
          P12 V1 does not kill apps, suspend arbitrary processes, disable
          services, or change scheduled tasks. Supported startup changes remain
          in the rollback-aware Startup Manager.
        </span>
      </section>

      {state.status === "idle" && (
        <section className="empty-health-state">
          <strong>No performance analysis has run in this session.</strong>
          <span>
            The scan is foreground-only and uses a small number of short
            samples.
          </span>
        </section>
      )}

      {state.status === "loading" && (
        <section className="scan-progress" aria-live="polite">
          <span className="scan-spinner" aria-hidden="true" />
          <div>
            <strong>Collecting repeated CPU/RAM/process evidence…</strong>
            <span>
              PC Manager is reading state only; no process or Windows setting is
              being changed.
            </span>
          </div>
        </section>
      )}

      {state.status === "error" && (
        <section className="info-callout health-error" aria-live="polite">
          <strong>Performance analysis unavailable</strong>
          <span>{state.message}</span>
          <button
            className="secondary-action"
            onClick={() => void runAnalysis()}
          >
            Retry
          </button>
        </section>
      )}

      {state.status === "ready" && (
        <>
          <section className="native-status-card">
            <div>
              <p className="eyebrow">Evidence window</p>
              <h3>{state.report.sampleCount} short samples</h3>
              <p className="muted">
                Findings require repeated visibility plus measurable CPU or
                memory use. This does not pretend to be a long benchmark.
              </p>
            </div>
            <span className="status-badge neutral">
              {findings.length} findings
            </span>
            <dl>
              <div>
                <dt>Average CPU</dt>
                <dd>
                  {formatOptimizerPercent(state.report.averageCpuPercent)}
                </dd>
              </div>
              <div>
                <dt>Average RAM</dt>
                <dd>
                  {formatOptimizerPercent(state.report.averageMemoryPercent)}
                </dd>
              </div>
              <div>
                <dt>Startup sources</dt>
                <dd>{state.report.startupEntriesAnalyzed}</dd>
              </div>
              <div>
                <dt>Running auto services</dt>
                <dd>{state.report.servicesAnalyzed}</dd>
              </div>
            </dl>
          </section>

          <section>
            <div className="section-heading">
              <div>
                <p className="eyebrow">Explainable recommendations</p>
                <h3>
                  {findings.length === 0
                    ? "No repeated high-cost candidate found"
                    : `${findings.length} review candidates`}
                </h3>
              </div>
              <span className="status-badge good">Read-only analysis</span>
            </div>

            {findings.length === 0 ? (
              <div className="empty-health-state">
                <strong>
                  No repeated resource-heavy candidate was proven.
                </strong>
                <span>
                  PC Manager will not manufacture an optimization just to show a
                  warning.
                </span>
              </div>
            ) : (
              <div className="p6-list">
                {findings.map((finding) => (
                  <article className="p6-item-card" key={finding.id}>
                    <div className="p6-item-main">
                      <div className="p6-item-heading">
                        <div>
                          <span className="startup-source">
                            {findingKindLabel(finding.kind)}
                          </span>
                          <h4>{finding.title}</h4>
                        </div>
                        <span
                          className={
                            finding.risk === "medium"
                              ? "status-badge warning"
                              : "status-badge neutral"
                          }
                        >
                          {finding.risk} risk
                        </span>
                      </div>
                      <p>{finding.summary}</p>
                      <ul>
                        {finding.evidence.map((item) => (
                          <li key={item}>{item}</li>
                        ))}
                      </ul>
                      <p className="muted">{finding.recommendation}</p>
                    </div>
                    <div className="p6-item-action">
                      {(finding.action === "reviewStartup" ||
                        finding.action === "reviewScheduledTask") && (
                        <button
                          className="secondary-action"
                          onClick={() => onNavigate("startup")}
                          type="button"
                        >
                          Open Startup Manager
                        </button>
                      )}
                      {(finding.action === "reviewAppSettings" ||
                        finding.action === "reviewService") && (
                        <button
                          className="secondary-action"
                          onClick={() => onNavigate("monitor")}
                          type="button"
                        >
                          Open Monitor
                        </button>
                      )}
                    </div>
                  </article>
                ))}
              </div>
            )}
          </section>

          <details className="p6-warning-box">
            <summary>Important limitations</summary>
            <ul>
              {state.report.limitations.map((item) => (
                <li key={item}>{item}</li>
              ))}
            </ul>
          </details>
        </>
      )}
    </div>
  );
}
