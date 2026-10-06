import { useRef, useState } from "react";

import {
  executeCleanupPlan,
  formatCleanupCategory,
  isCurrentScan,
  scanCleanupCandidates,
  type CleanupOperationRecord,
  type CleanupProviderSummary,
  type CleanupScanSummary,
} from "./smartClean";
import { formatBytes } from "./systemInventory";

type ScanState =
  | { status: "idle" }
  | { status: "scanning" }
  | { status: "cancelled" }
  | { status: "error"; message: string }
  | { status: "complete"; summary: CleanupScanSummary };

type ExecutionState =
  | { status: "idle" }
  | { status: "confirming" }
  | { status: "running" }
  | { status: "error"; message: string }
  | { status: "complete"; record: CleanupOperationRecord };

function providerStatus(provider: CleanupProviderSummary): string {
  if (!provider.available) {
    return "Unavailable";
  }
  if (provider.fileCount === 0) {
    return "Nothing found";
  }
  return "Candidates found";
}

function errorMessage(error: unknown, fallback: string): string {
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
  const [execution, setExecution] = useState<ExecutionState>({
    status: "idle",
  });
  const [includeRecycleBin, setIncludeRecycleBin] = useState(false);
  const [confirmed, setConfirmed] = useState(false);
  const generationRef = useRef(0);

  const startScan = () => {
    const generation = generationRef.current + 1;
    generationRef.current = generation;
    setExecution({ status: "idle" });
    setConfirmed(false);
    setState({ status: "scanning" });

    scanCleanupCandidates({ includeRecycleBin })
      .then((summary) => {
        if (!isCurrentScan(generation, generationRef.current)) {
          return;
        }
        setState({ status: "complete", summary });
      })
      .catch((error: unknown) => {
        if (!isCurrentScan(generation, generationRef.current)) {
          return;
        }

        setState({
          status: "error",
          message: errorMessage(
            error,
            "Smart Clean preview could not complete.",
          ),
        });
      });
  };

  const cancelScan = () => {
    if (state.status !== "scanning") {
      return;
    }

    generationRef.current += 1;
    setState({ status: "cancelled" });
  };

  const executePlan = async (summary: CleanupScanSummary) => {
    setExecution({ status: "running" });

    try {
      const record = await executeCleanupPlan(summary.planId);
      setExecution({ status: "complete", record });
    } catch (error: unknown) {
      setExecution({
        status: "error",
        message: errorMessage(error, "Cleanup execution could not complete."),
      });
    }
  };

  const summary = state.status === "complete" ? state.summary : null;
  const operation = execution.status === "complete" ? execution.record : null;

  return (
    <div className="page-stack cleaner-page">
      <section className="cleaner-hero">
        <div>
          <p className="eyebrow">Smart Clean — P4B</p>
          <h2>Preview first, then execute a revalidated native cleanup plan</h2>
          <p className="muted">
            PC Manager only deletes files that were discovered by built-in
            providers and still pass safety checks immediately before deletion.
            The frontend never submits arbitrary filesystem paths.
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
              disabled={execution.status === "running"}
              onClick={startScan}
            >
              {summary ? "Scan again" : "Scan cleanup candidates"}
            </button>
          )}
        </div>
      </section>

      <section className="cleaner-safety-card">
        <div>
          <span className="status-badge good">Safety gate enabled</span>
          <strong>Every planned file is checked again before deletion.</strong>
          <p>
            PC Manager revalidates the provider root, resolved path,
            reparse-point state, file size, modification state, and temp-file
            age rule. A changed or unsafe candidate is skipped instead of
            deleted.
          </p>
        </div>
        <label className="cleaner-option">
          <input
            checked={includeRecycleBin}
            disabled={
              state.status === "scanning" || execution.status === "running"
            }
            onChange={(event) => setIncludeRecycleBin(event.target.checked)}
            type="checkbox"
          />
          <span>
            <strong>Include Recycle Bin check</strong>
            <small>
              Still unavailable in P4B until its native provider is separately
              verified. This option never empties the Recycle Bin.
            </small>
          </span>
        </label>
      </section>

      {state.status === "scanning" && (
        <section className="scan-progress" aria-live="polite">
          <span className="scan-spinner" aria-hidden="true" />
          <div>
            <strong>Scanning explicit temp and cache roots…</strong>
            <span>
              Junctions, symlinks, reparse points, and arbitrary user folders
              are excluded.
            </span>
          </div>
        </section>
      )}

      {state.status === "cancelled" && (
        <section className="info-callout" aria-live="polite">
          <strong>Preview scan cancelled</strong>
          <span>Any late result is discarded. Nothing was deleted.</span>
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

      {summary ? (
        <>
          <section className="cleaner-total-card">
            <div>
              <p className="eyebrow">Preview total</p>
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
                  Exactly where the preview came from
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
                  {provider.warnings.length > 0 && (
                    <ul>
                      {provider.warnings.map((warning) => (
                        <li key={warning}>{warning}</li>
                      ))}
                    </ul>
                  )}
                </article>
              ))}
            </div>
          </section>

          {summary.warnings.length > 0 && (
            <section className="inventory-warning-list">
              <p className="eyebrow">Scan warnings</p>
              <h3>Some provider items could not be inspected</h3>
              <ul>
                {summary.warnings.map((warning, index) => (
                  <li key={`${warning.providerId}-${index}`}>
                    <strong>{warning.providerId}</strong>
                    <span>{warning.message}</span>
                  </li>
                ))}
              </ul>
            </section>
          )}

          {execution.status === "idle" && (
            <section className="cleaner-locked-action">
              <div>
                <strong>Ready for explicit confirmation</strong>
                <span>
                  Cache and temp deletion is generally not restorable. The plan
                  can be executed only once; run a new scan for another cleanup.
                </span>
              </div>
              <button
                disabled={
                  !summary.executionAvailable || summary.totalFiles === 0
                }
                onClick={() => {
                  setConfirmed(false);
                  setExecution({ status: "confirming" });
                }}
                type="button"
              >
                Review cleanup
              </button>
            </section>
          )}

          {execution.status === "confirming" && (
            <section
              className="cleanup-confirmation"
              role="dialog"
              aria-modal="true"
            >
              <p className="eyebrow">Confirmation required</p>
              <h3>
                Delete up to {formatBytes(summary.totalBytes)} from{" "}
                {summary.totalFiles.toLocaleString()} planned files?
              </h3>
              <p>
                Files that changed after the scan, moved outside their approved
                root, became reparse points, or fail the provider rule will be
                skipped. Successful cache/temp deletion is not automatically
                reversible.
              </p>
              <label>
                <input
                  checked={confirmed}
                  onChange={(event) => setConfirmed(event.target.checked)}
                  type="checkbox"
                />
                <span>
                  I understand that successfully deleted cache/temp files are
                  not restorable by PC Manager.
                </span>
              </label>
              <div className="confirmation-actions">
                <button
                  className="secondary-action"
                  onClick={() => {
                    setConfirmed(false);
                    setExecution({ status: "idle" });
                  }}
                >
                  Cancel
                </button>
                <button
                  className="danger-action"
                  disabled={!confirmed}
                  onClick={() => void executePlan(summary)}
                >
                  Delete planned files
                </button>
              </div>
            </section>
          )}

          {execution.status === "running" && (
            <section className="scan-progress" aria-live="polite">
              <span className="scan-spinner" aria-hidden="true" />
              <div>
                <strong>Revalidating and deleting eligible candidates…</strong>
                <span>
                  Failures are isolated per file and recorded in the operation
                  result.
                </span>
              </div>
            </section>
          )}

          {execution.status === "error" && (
            <section className="info-callout health-error" aria-live="polite">
              <strong>Cleanup execution could not complete</strong>
              <span>{execution.message}</span>
              <span>Run a new preview scan before trying again.</span>
            </section>
          )}

          {operation && (
            <section className="cleanup-result-card" aria-live="polite">
              <div>
                <p className="eyebrow">Cleanup result</p>
                <h3>{formatBytes(operation.deletedBytes)} deleted</h3>
                <p className="muted">
                  {operation.deletedFiles.toLocaleString()} files deleted,{" "}
                  {operation.failedFiles.toLocaleString()} skipped/failed.
                </p>
              </div>
              <dl>
                <div>
                  <dt>Requested</dt>
                  <dd>{formatBytes(operation.requestedBytes)}</dd>
                </div>
                <div>
                  <dt>Rollback</dt>
                  <dd>{operation.rollbackLabel}</dd>
                </div>
                <div>
                  <dt>Operation ID</dt>
                  <dd>{operation.operationId}</dd>
                </div>
              </dl>
              {operation.errors.length > 0 && (
                <details>
                  <summary>
                    Show {operation.errors.length} recorded execution warning(s)
                  </summary>
                  <ul>
                    {operation.errors.map((error, index) => (
                      <li key={`${error.providerId}-${error.code}-${index}`}>
                        <strong>{error.providerId}</strong>: {error.message}
                      </li>
                    ))}
                  </ul>
                </details>
              )}
            </section>
          )}
        </>
      ) : (
        state.status !== "scanning" && (
          <section className="empty-health-state">
            <strong>No Smart Clean preview yet.</strong>
            <span>
              Scan first. PC Manager never executes cleanup without a native
              plan and an explicit confirmation.
            </span>
          </section>
        )
      )}
    </div>
  );
}
