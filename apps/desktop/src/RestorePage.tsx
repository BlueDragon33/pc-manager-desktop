import { useEffect, useState } from "react";

import {
  listCleanupOperations,
  type CleanupOperationRecord,
} from "./smartClean";
import { formatBytes } from "./systemInventory";

type HistoryState =
  | { status: "loading" }
  | { status: "ready"; records: CleanupOperationRecord[] }
  | { status: "error"; message: string };

export function RestorePage() {
  const [state, setState] = useState<HistoryState>({ status: "loading" });

  const load = () => {
    setState({ status: "loading" });
    listCleanupOperations()
      .then((records) => setState({ status: "ready", records }))
      .catch((error: unknown) => {
        setState({
          status: "error",
          message:
            error instanceof Error
              ? error.message
              : "Operation history could not be loaded.",
        });
      });
  };

  useEffect(() => {
    load();
  }, []);

  return (
    <div className="page-stack restore-page">
      <section className="restore-hero">
        <div>
          <p className="eyebrow">Restore Center</p>
          <h2>Operation history and rollback capability</h2>
          <p className="muted">
            Restore Center does not pretend deleted cache files can be
            recovered. P4B records cleanup operations and clearly labels whether
            a safe rollback exists.
          </p>
        </div>
        <button className="secondary-action" onClick={load}>
          Refresh
        </button>
      </section>

      {state.status === "loading" && (
        <section className="scan-progress">
          <span className="scan-spinner" aria-hidden="true" />
          <div>
            <strong>Loading local operation history…</strong>
            <span>No file contents or personal paths are stored here.</span>
          </div>
        </section>
      )}

      {state.status === "error" && (
        <section className="info-callout health-error">
          <strong>Operation history unavailable</strong>
          <span>{state.message}</span>
          <button className="secondary-action" onClick={load}>
            Retry
          </button>
        </section>
      )}

      {state.status === "ready" && state.records.length === 0 && (
        <section className="empty-health-state">
          <strong>No maintenance operations have been recorded yet.</strong>
          <span>
            Completed Smart Clean executions will appear here with exact result
            totals and rollback availability.
          </span>
        </section>
      )}

      {state.status === "ready" && state.records.length > 0 && (
        <section
          className="operation-history"
          aria-labelledby="history-heading"
        >
          <div className="section-heading">
            <div>
              <p className="eyebrow">Local audit</p>
              <h3 id="history-heading">
                {state.records.length} recorded operation(s)
              </h3>
            </div>
          </div>

          <div className="operation-list">
            {state.records.map((record) => (
              <article className="operation-card" key={record.operationId}>
                <div className="operation-card-heading">
                  <div>
                    <span>Smart Clean</span>
                    <h4>{formatBytes(record.deletedBytes)} deleted</h4>
                  </div>
                  <span
                    className={
                      record.rollbackAvailable
                        ? "status-badge good"
                        : "status-badge neutral"
                    }
                  >
                    {record.rollbackLabel}
                  </span>
                </div>

                <dl>
                  <div>
                    <dt>Completed</dt>
                    <dd>
                      {new Date(record.completedAtEpochMs).toLocaleString()}
                    </dd>
                  </div>
                  <div>
                    <dt>Files</dt>
                    <dd>
                      {record.deletedFiles.toLocaleString()} deleted /{" "}
                      {record.failedFiles.toLocaleString()} skipped
                    </dd>
                  </div>
                  <div>
                    <dt>Requested</dt>
                    <dd>{formatBytes(record.requestedBytes)}</dd>
                  </div>
                  <div>
                    <dt>Operation ID</dt>
                    <dd>{record.operationId}</dd>
                  </div>
                </dl>

                {record.errors.length > 0 && (
                  <details>
                    <summary>
                      {record.errors.length} recorded warning(s)
                    </summary>
                    <ul>
                      {record.errors.map((error, index) => (
                        <li
                          key={`${record.operationId}-${error.code}-${index}`}
                        >
                          <strong>{error.providerId}</strong>: {error.message}
                        </li>
                      ))}
                    </ul>
                  </details>
                )}
              </article>
            ))}
          </div>
        </section>
      )}
    </div>
  );
}
