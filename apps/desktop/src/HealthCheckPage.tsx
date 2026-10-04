import { useRef, useState } from "react";

import {
  formatHealthCategory,
  loadLatestHealthReport,
  runHealthCheck,
  saveLatestHealthReport,
  type HealthCategoryResult,
  type HealthFinding,
  type HealthReport,
} from "./healthCheck";
import { formatBytes } from "./systemInventory";

type ScanState =
  | { status: "idle"; report: HealthReport | null }
  | { status: "scanning"; previous: HealthReport | null }
  | { status: "cancelled"; report: HealthReport | null }
  | { status: "error"; report: HealthReport | null; message: string }
  | { status: "complete"; report: HealthReport };

function reportFromState(state: ScanState): HealthReport | null {
  if (state.status === "complete") {
    return state.report;
  }
  if (state.status === "scanning") {
    return state.previous;
  }
  return state.report;
}

function statusLabel(result: HealthCategoryResult): string {
  switch (result.status) {
    case "good":
      return "Good";
    case "attention":
      return "Attention";
    case "critical":
      return "Critical";
    case "unavailable":
      return "Unavailable";
  }
}

function evidenceWithFriendlyBytes(evidence: string): string {
  const match = evidence.match(/\((\d+) of (\d+) bytes(?: available)?\)/);
  if (!match) {
    return evidence;
  }

  const available = Number(match[1]);
  const total = Number(match[2]);

  return evidence.replace(
    match[0],
    `(${formatBytes(available)} of ${formatBytes(total)})`,
  );
}

function FindingCard({ finding }: { finding: HealthFinding }) {
  return (
    <article className={`finding-card severity-${finding.severity}`}>
      <div className="finding-heading">
        <div>
          <span className="finding-category">
            {formatHealthCategory(finding.category)}
          </span>
          <h4>{finding.title}</h4>
        </div>
        <span className={`status-badge ${finding.severity}`}>
          {finding.severity}
        </span>
      </div>
      <p>{finding.description}</p>
      <dl className="finding-details">
        <div>
          <dt>Evidence</dt>
          <dd>{evidenceWithFriendlyBytes(finding.evidence)}</dd>
        </div>
        <div>
          <dt>Recommendation</dt>
          <dd>{finding.recommendedAction}</dd>
        </div>
        <div>
          <dt>Action in P3</dt>
          <dd>Read-only — no automatic change is available.</dd>
        </div>
      </dl>
    </article>
  );
}

export function HealthCheckPage() {
  const [state, setState] = useState<ScanState>(() => {
    const report = loadLatestHealthReport();
    return report
      ? { status: "complete", report }
      : { status: "idle", report: null };
  });
  const generationRef = useRef(0);

  const startScan = () => {
    const previous = reportFromState(state);
    const generation = generationRef.current + 1;
    generationRef.current = generation;
    setState({ status: "scanning", previous });

    runHealthCheck()
      .then((report) => {
        if (generationRef.current !== generation) {
          return;
        }
        saveLatestHealthReport(report);
        setState({ status: "complete", report });
      })
      .catch((error: unknown) => {
        if (generationRef.current !== generation) {
          return;
        }
        setState({
          status: "error",
          report: previous,
          message:
            typeof error === "object" &&
            error !== null &&
            "message" in error &&
            typeof (error as { message?: unknown }).message === "string"
              ? (error as { message: string }).message
              : error instanceof Error
                ? error.message
                : "Health Check could not complete.",
        });
      });
  };

  const cancelScan = () => {
    if (state.status !== "scanning") {
      return;
    }

    // P3 deliberately does not kill PowerShell or arbitrary OS processes.
    // Invalidating the generation safely discards a late native result and
    // prevents cancelled scans from being rendered or persisted.
    generationRef.current += 1;
    setState({ status: "cancelled", report: state.previous });
  };

  const report = reportFromState(state);

  return (
    <div className="page-stack health-page">
      <section className="health-hero">
        <div>
          <p className="eyebrow">Explainable health check</p>
          <h2>
            {state.status === "scanning"
              ? "Scanning this PC…"
              : report
                ? "Latest completed scan"
                : "Ready for the first scan"}
          </h2>
          <p className="muted">
            P3 evaluates only evidence that PC Manager can currently verify.
            Unsupported checks stay unavailable instead of being guessed.
          </p>
        </div>

        <div className="health-actions">
          {state.status === "scanning" ? (
            <button className="secondary-action" onClick={cancelScan}>
              Cancel
            </button>
          ) : (
            <button className="primary-action" onClick={startScan}>
              {report ? "Scan again" : "Scan now"}
            </button>
          )}
        </div>
      </section>

      {state.status === "scanning" && (
        <section className="scan-progress" aria-live="polite">
          <span className="scan-spinner" aria-hidden="true" />
          <div>
            <strong>Collecting a fresh Windows snapshot</strong>
            <span>
              Storage and memory checks are read-only. No cleanup or
              optimization runs during Health Check.
            </span>
          </div>
        </section>
      )}

      {state.status === "cancelled" && (
        <section className="info-callout" aria-live="polite">
          <strong>Scan cancelled</strong>
          <span>
            A late native result will be ignored and will not replace the latest
            completed report.
          </span>
        </section>
      )}

      {state.status === "error" && (
        <section className="info-callout health-error" aria-live="polite">
          <strong>Health Check could not complete</strong>
          <span>{state.message}</span>
          <button className="secondary-action" onClick={startScan}>
            Retry
          </button>
        </section>
      )}

      {report ? (
        <>
          <section className="health-score-card">
            <div className="health-score-ring" aria-label="Health score">
              <strong>{report.score ?? "—"}</strong>
              <span>/ 100</span>
            </div>
            <div>
              <p className="eyebrow">Current score</p>
              <h3>
                {report.coveragePercent < 100
                  ? "Partial health score"
                  : "Health score"}
              </h3>
              <p className="muted">
                {report.supportedCategories} of {report.totalCategories}{" "}
                categories are supported in this version (
                {report.coveragePercent}% coverage). Unsupported categories do
                not reduce the score.
              </p>
            </div>
            <span className="status-badge neutral">
              {new Date(report.collectedAtEpochMs).toLocaleString()}
            </span>
          </section>

          <section aria-labelledby="health-category-results">
            <div className="section-heading">
              <div>
                <p className="eyebrow">Categories</p>
                <h3 id="health-category-results">Evidence by category</h3>
              </div>
            </div>
            <div className="health-category-grid">
              {report.categoryResults.map((result) => (
                <article
                  className={`health-category-card status-${result.status}`}
                  key={result.category}
                >
                  <div className="health-category-title">
                    <h4>{formatHealthCategory(result.category)}</h4>
                    <span>{statusLabel(result)}</span>
                  </div>
                  <strong className="category-score">
                    {result.score === null ? "—" : `${result.score}/100`}
                  </strong>
                  <p>{result.summary}</p>
                </article>
              ))}
            </div>
          </section>

          <section aria-labelledby="health-findings">
            <div className="section-heading">
              <div>
                <p className="eyebrow">Findings</p>
                <h3 id="health-findings">
                  {report.findings.length
                    ? `${report.findings.length} finding(s) need context`
                    : "No supported checks need attention"}
                </h3>
              </div>
            </div>

            {report.findings.length ? (
              <div className="finding-list">
                {report.findings.map((finding) => (
                  <FindingCard finding={finding} key={finding.id} />
                ))}
              </div>
            ) : (
              <div className="empty-health-state">
                <strong>Supported checks look comfortable.</strong>
                <span>
                  Security, Updates, and Privacy remain unavailable in P3 and
                  are not included in this score.
                </span>
              </div>
            )}
          </section>

          {report.inventoryWarnings.length > 0 && (
            <section className="inventory-warning-list">
              <p className="eyebrow">Inventory warnings</p>
              <h3>Some Windows sources were unavailable</h3>
              <ul>
                {report.inventoryWarnings.map((warning) => (
                  <li key={`${warning.source}:${warning.message}`}>
                    <strong>{warning.source}</strong>
                    <span>{warning.message}</span>
                  </li>
                ))}
              </ul>
            </section>
          )}
        </>
      ) : (
        state.status !== "scanning" && (
          <section className="empty-health-state">
            <strong>No completed Health Check yet.</strong>
            <span>
              Run a scan to evaluate real storage and memory evidence. Nothing
              is changed on the PC.
            </span>
          </section>
        )
      )}
    </div>
  );
}
