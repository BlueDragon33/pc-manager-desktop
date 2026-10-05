import { useRef, useState } from "react";

import { saveCleanupOperation } from "./operationHistory";
import {
  executeCleanupPlan,
  formatCleanupCategory,
  isCurrentScan,
  scanCleanupCandidates,
  type CleanupExecutionResult,
  type CleanupProviderSummary,
  type CleanupScanSummary,
} from "./smartClean";
import { formatBytes } from "./systemInventory";

type ScanState =
  | { status: "idle" }
  | { status: "scanning" }
  | { status: "cancelled" }
  | { status: "error"; message: string }
  | { status: "complete"; summary: CleanupScanSummary }
  | { status: "executing"; summary: CleanupScanSummary }
  | {
      status: "executed";
      summary: CleanupScanSummary;
      result: CleanupExecutionResult;
    };

function providerStatus(provider: CleanupProviderSummary): string {
  if (!provider.available) return "Unavailable";
  if (provider.fileCount === 0) return "Nothing found";
  return "Candidates found";
}

function messageFromError(error: unknown, fallback: string): string {
  if (
    typeof error === "object" &&
    error !== null &&
    "message" in error &&
    typeof (error as { message?: unknown }).message === "string"
  ) {
    return (error as { message: string }).message;
  }
  return error instanceof Error ? error.message : fallback;
}

export function SmartCleanPage() {
  const [state, setState] = useState<ScanState>({ status: "idle" });
  const [includeRecycleBin, setIncludeRecycleBin] = useState(false);
  const [executionError, setExecutionError] = useState<string | null>(null);
  const generationRef = useRef(0);

  const startScan = () => {
    const generation = generationRef.current + 1;
    generationRef.current = generation;
    setExecutionError(null);
    setState({ status: "scanning" });

    scanCleanupCandidates({ includeRecycleBin })
      .then((summary) => {
        if (!isCurrentScan(generation, generationRef.current)) return;
        setState({ status: "complete", summary });
      })
      .catch((error: unknown) => {
        if (!isCurrentScan(generation, generationRef.current)) return;
        setState({
          status: "error",
          message: messageFromError(
            error,
            "Smart Clean preview could not complete.",
          ),
        });
      });
  };

  const cancelScan = () => {
    if (state.status !== "scanning") return;
    generationRef.current += 1;
    setState({ status: "cancelled" });
  };

  const summary =
    state.status === "complete" ||
    state.status === "executing" ||
    state.status === "executed"
      ? state.summary
      : null;

  const executePlan = async () => {
    if (
      !summary ||
      !summary.executionAvailable ||
      state.status === "executing"
    ) {
      return;
    }

    const confirmed = window.confirm(
      "Delete the files in this verified cleanup plan?\n\n" +
        "This action deletes temporary/cache files only, but it is not automatically restorable. " +
        "Files that changed since the preview will be skipped.",
    );
    if (!confirmed) return;

    setExecutionError(null);
    setState({ status: "executing", summary });

    try {
      const result = await executeCleanupPlan(summary.planId);
      saveCleanupOperation(result);
      setState({ status: "executed", summary, result });
    } catch (error: unknown) {
      setExecutionError(
        messageFromError(error, "The cleanup plan could not be executed."),
      );
      setState({ status: "complete", summary });
    }
  };

  const result = state.status === "executed" ? state.result : null;

  return (
    <div className="page-stack cleaner-page">
      <section className="cleaner-hero">
        <div>
          <p className="eyebrow">Smart Clean — P4B</p>
          <h2>Preview first, then execute the verified native plan</h2>
          <p className="muted">
            Only candidates created by the native allow-listed scanner can be
            deleted. Every file is revalidated immediately before removal.
          </p>
        </div>
        <div className="cleaner-actions">
          {state.status === "scanning" ? (
            <button className="secondary-action" onClick={cancelScan}>
              Cancel scan
            </button>
          ) : (
            <button
              className="primary-action"
              disabled={state.status === "executing"}
              onClick={startScan}
            >
              {summary ? "Scan again" : "Scan cleanup candidates"}
            </button>
          )}
        </div>
      </section>

      <section className="cleaner-safety-card">
        <div>
          <span className="status-badge good">Plan-gated execution</span>
          <strong>The UI cannot submit arbitrary paths for deletion.</strong>
          <p>
            P4B executes only the native plan ID produced by the latest scan.
            Changed, missing, reparse, or escaped files are skipped and recorded
            as failures instead of being forced.
          </p>
        </div>
        <label className="cleaner-option">
          <input
            checked={includeRecycleBin}
            disabled={
              state.status === "scanning" || state.status === "executing"
            }
            onChange={(event) => setIncludeRecycleBin(event.target.checked)}
            type="checkbox"
          />
          <span>
            <strong>Include Recycle Bin check</strong>
            <small>
              Still unavailable until a verified native provider is implemented.
            </small>
          </span>
        </label>
      </section>

      {state.status === "scanning" && (
        <section className="scan-progress" aria-live="polite">
          <span className="scan-spinner" aria-hidden="true" />
          <div>
            <strong>Scanning explicit temp and cache roots…</strong>
            <span>No files are changed during the preview scan.</span>
          </div>
        </section>
      )}

      {state.status === "executing" && (
        <section className="scan-progress" aria-live="polite">
          <span className="scan-spinner" aria-hidden="true" />
          <div>
            <strong>Revalidating and cleaning the native plan…</strong>
            <span>
              Each candidate must still match its original provider root, type,
              size, and modification state before deletion.
            </span>
          </div>
        </section>
      )}

      {state.status === "cancelled" && (
        <section className="info-callout" aria-live="polite">
          <strong>Preview scan cancelled</strong>
          <span>Nothing was modified or deleted.</span>
        </section>
      )}

      {state.status === "error" && (
        <section className="info-callout health-error" aria-live="polite">
          <strong>Smart Clean preview could not complete</strong>
          <span>{state.message}</span>
          <button className="secondary-action" onClick={startScan}>
            Retry
          </button>
        </section>
      )}

      {executionError && (
        <section className="info-callout health-error" aria-live="polite">
          <strong>Cleanup execution failed</strong>
          <span>{executionError}</span>
        </section>
      )}

      {result && (
        <section className="cleanup-result-card">
          <div>
            <p className="eyebrow">Execution result</p>
            <h3>{formatBytes(result.deletedBytes)} removed</h3>
            <p className="muted">
              {result.deletedFiles.toLocaleString()} of{" "}
              {result.requestedFiles.toLocaleString()} planned files were
              deleted.
              {result.failedItems > 0
                ? ` ${result.failedItems.toLocaleString()} item(s) were safely skipped or failed.`
                : " All planned candidates passed revalidation."}
            </p>
          </div>
          <span className="status-badge warning">Not restorable</span>
        </section>
      )}

      {summary ? (
        <>
          <section className="cleaner-total-card">
            <div>
              <p className="eyebrow">Plan total</p>
              <strong className="cleaner-total-bytes">
                {formatBytes(summary.totalBytes)}
              </strong>
              <span>{summary.totalFiles.toLocaleString()} candidate files</span>
            </div>
            <dl>
              <div>
                <dt>Plan ID</dt>
                <dd>{summary.planId}</dd>
              </div>
              <div>
                <dt>Collected</dt>
                <dd>{new Date(summary.collectedAtEpochMs).toLocaleString()}</dd>
              </div>
              <div>
                <dt>Execution</dt>
                <dd>
                  {summary.executionAvailable ? "Available" : "Unavailable"}
                </dd>
              </div>
            </dl>
          </section>

          <section aria-labelledby="cleaner-provider-heading">
            <div className="section-heading">
              <div>
                <p className="eyebrow">Providers</p>
                <h3 id="cleaner-provider-heading">
                  Exactly where the plan came from
                </h3>
              </div>
            </div>
            <div className="cleaner-provider-grid">
              {summary.providers.map((provider) => (
                <article
                  className={
                    provider.available
                      ? "cleaner-provider-card"
                      : "cleaner-provider-card provider-unavailable"
                  }
                  key={provider.providerId}
                >
                  <div className="cleaner-provider-heading">
                    <div>
                      <span>{formatCleanupCategory(provider.category)}</span>
                      <h4>{provider.displayName}</h4>
                    </div>
                    <span className="status-badge neutral">
                      {providerStatus(provider)}
                    </span>
                  </div>
                  <div className="cleaner-provider-metrics">
                    <strong>{formatBytes(provider.bytes)}</strong>
                    <span>{provider.fileCount.toLocaleString()} files</span>
                  </div>
                  <p>{provider.description}</p>
                  <small>
                    {provider.reversible
                      ? "Rollback supported"
                      : "Deletion is not automatically restorable"}
                  </small>
                </article>
              ))}
            </div>
          </section>

          <section className="cleaner-locked-action">
            <div>
              <strong>
                {result ? "Cleanup completed" : "Ready for verified execution"}
              </strong>
              <span>
                {result
                  ? result.rollbackSummary
                  : "A confirmation dialog is required. Files that no longer match the preview are skipped."}
              </span>
            </div>
            <button
              className="danger-action"
              disabled={
                !summary.executionAvailable ||
                state.status === "executing" ||
                Boolean(result)
              }
              onClick={executePlan}
              type="button"
            >
              {state.status === "executing"
                ? "Cleaning…"
                : result
                  ? "Plan executed"
                  : "Clean planned items"}
            </button>
          </section>
        </>
      ) : (
        state.status !== "scanning" && (
          <section className="empty-health-state">
            <strong>No Smart Clean preview yet.</strong>
            <span>Run a scan before any cleanup can be executed.</span>
          </section>
        )
      )}
    </div>
  );
}
