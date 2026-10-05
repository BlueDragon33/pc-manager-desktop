import { useState } from "react";

import { loadCleanupOperations } from "./operationHistory";
import { formatBytes } from "./systemInventory";

export function RestoreCenterPage() {
  const [operations, setOperations] = useState(() => loadCleanupOperations());

  return (
    <div className="page-stack restore-page">
      <section className="restore-hero">
        <div>
          <p className="eyebrow">Restore Center</p>
          <h2>Operation history and rollback truth</h2>
          <p className="muted">
            P4B records completed Smart Clean operations locally. Cache and
            temporary-file deletion is intentionally marked as not restorable.
          </p>
        </div>
        <button
          className="secondary-action"
          onClick={() => setOperations(loadCleanupOperations())}
        >
          Refresh
        </button>
      </section>

      {operations.length === 0 ? (
        <section className="empty-health-state">
          <strong>No cleanup operations recorded yet.</strong>
          <span>
            Completed Smart Clean executions will appear here with exact results.
          </span>
        </section>
      ) : (
        <div className="operation-list">
          {operations.map((operation) => (
            <article className="operation-card" key={operation.operationId}>
              <div className="operation-heading">
                <div>
                  <span className="finding-category">Smart Clean</span>
                  <h3>{new Date(operation.completedAtEpochMs).toLocaleString()}</h3>
                </div>
                <span className="status-badge warning">Not restorable</span>
              </div>
              <div className="operation-metrics">
                <div>
                  <span>Deleted</span>
                  <strong>{formatBytes(operation.deletedBytes)}</strong>
                </div>
                <div>
                  <span>Files</span>
                  <strong>
                    {operation.deletedFiles.toLocaleString()} /{" "}
                    {operation.requestedFiles.toLocaleString()}
                  </strong>
                </div>
                <div>
                  <span>Failed</span>
                  <strong>{operation.failedItems.toLocaleString()}</strong>
                </div>
              </div>
              <p>{operation.rollbackSummary}</p>
              {operation.errors.length > 0 && (
                <details>
                  <summary>Show execution errors</summary>
                  <ul>
                    {operation.errors.map((error, index) => (
                      <li key={operation.operationId + "-" + index}>{error}</li>
                    ))}
                  </ul>
                </details>
              )}
            </article>
          ))}
        </div>
      )}
    </div>
  );
}
