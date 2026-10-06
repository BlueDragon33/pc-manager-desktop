import { useMemo, useRef, useState } from "react";

import {
  canSelectDuplicate,
  cancelFilesystemScan,
  deleteDuplicateFiles,
  duplicateWaste,
  parseExclusions,
  scanDuplicates,
  selectScanRoot,
  type DuplicateDeleteResult,
  type DuplicateGroup,
  type DuplicateScanSummary,
  type ScanRootSelection,
} from "./duplicates";

type ScanState =
  | { status: "idle" }
  | { status: "scanning"; requestId: string }
  | { status: "ready"; summary: DuplicateScanSummary }
  | { status: "error"; message: string };

type DeleteState =
  | { status: "idle" }
  | { status: "confirming" }
  | { status: "running" }
  | { status: "complete"; result: DuplicateDeleteResult }
  | { status: "error"; message: string };

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

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  const units = ["KB", "MB", "GB", "TB"];
  let value = bytes / 1024;
  let unit = units[0];
  for (let index = 1; index < units.length && value >= 1024; index += 1) {
    value /= 1024;
    unit = units[index];
  }
  return `${value >= 10 ? value.toFixed(1) : value.toFixed(2)} ${unit}`;
}

function newRequestId(prefix: string): string {
  return `${prefix}-${crypto.randomUUID()}`;
}

export function DuplicateFinderPage() {
  const [roots, setRoots] = useState<ScanRootSelection[]>([]);
  const [exclusions, setExclusions] = useState("node_modules, .git, target");
  const [minimumKb, setMinimumKb] = useState(1);
  const [scanState, setScanState] = useState<ScanState>({ status: "idle" });
  const [deleteState, setDeleteState] = useState<DeleteState>({
    status: "idle",
  });
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const generation = useRef(0);

  const summary = scanState.status === "ready" ? scanState.summary : null;
  const totalWaste = useMemo(
    () => (summary ? duplicateWaste(summary.duplicateGroups) : 0),
    [summary],
  );
  const selectedBytes = useMemo(() => {
    if (!summary) return 0;
    const selected = selectedIds;
    return summary.duplicateGroups.reduce(
      (total, group) =>
        total +
        group.files
          .filter((file) => selected.has(file.id))
          .reduce((sum, file) => sum + file.bytes, 0),
      0,
    );
  }, [selectedIds, summary]);

  const addRoot = async () => {
    try {
      const selected = await selectScanRoot();
      if (!selected) return;
      setRoots((current) =>
        current.some((root) => root.id === selected.id)
          ? current
          : [...current, selected],
      );
    } catch (error: unknown) {
      setScanState({
        status: "error",
        message: errorMessage(error, "The folder picker could not be opened."),
      });
    }
  };

  const startScan = async () => {
    const requestId = newRequestId("duplicates");
    const currentGeneration = generation.current + 1;
    generation.current = currentGeneration;
    setSelectedIds(new Set());
    setDeleteState({ status: "idle" });
    setScanState({ status: "scanning", requestId });

    try {
      const result = await scanDuplicates(
        requestId,
        roots.map((root) => root.id),
        {
          minFileBytes: Math.max(1, Math.round(minimumKb * 1024)),
          excludedDirectoryNames: parseExclusions(exclusions),
          maxGroups: 200,
          maxFilesPerGroup: 50,
        },
      );
      if (generation.current === currentGeneration) {
        setScanState({ status: "ready", summary: result });
      }
    } catch (error: unknown) {
      if (generation.current === currentGeneration) {
        const message = errorMessage(error, "Duplicate scan failed.");
        if (!message.toLocaleLowerCase().includes("cancel")) {
          setScanState({ status: "error", message });
        } else {
          setScanState({ status: "idle" });
        }
      }
    }
  };

  const cancelScan = async () => {
    if (scanState.status !== "scanning") return;
    const { requestId } = scanState;
    generation.current += 1;
    setScanState({ status: "idle" });
    await cancelFilesystemScan(requestId).catch(() => false);
  };

  const toggleFile = (group: DuplicateGroup, fileId: string) => {
    setSelectedIds((current) => {
      if (!current.has(fileId) && !canSelectDuplicate(group, current, fileId)) {
        return current;
      }
      const next = new Set(current);
      if (next.has(fileId)) next.delete(fileId);
      else next.add(fileId);
      return next;
    });
  };

  const executeDelete = async () => {
    if (!summary || selectedIds.size === 0) return;
    setDeleteState({ status: "running" });
    try {
      const result = await deleteDuplicateFiles(
        summary.scanId,
        Array.from(selectedIds),
      );
      setSelectedIds(new Set());
      setScanState({ status: "idle" });
      setDeleteState({ status: "complete", result });
    } catch (error: unknown) {
      setDeleteState({
        status: "error",
        message: errorMessage(error, "Selected duplicate files were not deleted."),
      });
    }
  };

  return (
    <div className="page-stack p6-page">
      <section className="p6-hero">
        <div>
          <p className="eyebrow">Duplicate Finder — P6B</p>
          <h2>Prove duplicates by size, partial hash, then full hash</h2>
          <p className="muted">
            A filename match is never enough. Only full-hash verified files are
            shown as duplicates, and deletion is always manual.
          </p>
        </div>
        <button className="secondary-action" onClick={() => void addRoot()}>
          Add folder
        </button>
      </section>

      <section className="p6-scan-config">
        <div>
          <strong>Selected folders</strong>
          {roots.length === 0 ? (
            <p className="muted">Choose at least one folder to scan.</p>
          ) : (
            <div className="root-chip-list">
              {roots.map((root) => (
                <button
                  className="root-chip"
                  key={root.id}
                  onClick={() =>
                    setRoots((current) =>
                      current.filter((item) => item.id !== root.id),
                    )
                  }
                  title="Remove this folder from the next scan"
                  type="button"
                >
                  <span>{root.displayPath}</span>
                  <b aria-hidden="true">×</b>
                </button>
              ))}
            </div>
          )}
        </div>
        <label>
          <span>Excluded directory names</span>
          <input
            onChange={(event) => setExclusions(event.target.value)}
            placeholder="node_modules, .git, target"
            value={exclusions}
          />
        </label>
        <label>
          <span>Minimum file size (KB)</span>
          <input
            min="1"
            onChange={(event) => setMinimumKb(Number(event.target.value) || 1)}
            type="number"
            value={minimumKb}
          />
        </label>
        <div className="p6-scan-actions">
          {scanState.status === "scanning" ? (
            <button className="secondary-action" onClick={() => void cancelScan()}>
              Cancel scan
            </button>
          ) : (
            <button
              className="primary-action"
              disabled={roots.length === 0}
              onClick={() => void startScan()}
            >
              Scan for duplicates
            </button>
          )}
        </div>
      </section>

      {scanState.status === "scanning" && (
        <section className="scan-progress">
          <span className="scan-spinner" aria-hidden="true" />
          <div>
            <strong>Scanning and hashing candidate files…</strong>
            <span>
              Reparse points are skipped. Full hashing runs only after cheaper
              candidate stages.
            </span>
          </div>
        </section>
      )}

      {scanState.status === "error" && (
        <section className="info-callout health-error">
          <strong>Duplicate scan failed</strong>
          <span>{scanState.message}</span>
        </section>
      )}

      {deleteState.status === "complete" && (
        <section className="info-callout startup-success">
          <strong>Duplicate deletion finished</strong>
          <span>
            Deleted {deleteState.result.deletedFiles} of{" "}
            {deleteState.result.requestedFiles} selected files ·{" "}
            {formatBytes(deleteState.result.deletedBytes)} reclaimed.
          </span>
          <span>
            {deleteState.result.failedFiles > 0
              ? `${deleteState.result.failedFiles} files were skipped after revalidation.`
              : "Every selected file passed revalidation before deletion."}
          </span>
        </section>
      )}

      {deleteState.status === "error" && (
        <section className="info-callout health-error">
          <strong>Duplicate deletion stopped</strong>
          <span>{deleteState.message}</span>
        </section>
      )}

      {summary && (
        <>
          <section className="p6-summary-grid">
            <article>
              <span>Files scanned</span>
              <strong>{summary.scannedFiles.toLocaleString()}</strong>
            </article>
            <article>
              <span>Data inspected</span>
              <strong>{formatBytes(summary.scannedBytes)}</strong>
            </article>
            <article>
              <span>Verified groups</span>
              <strong>{summary.duplicateGroups.length}</strong>
            </article>
            <article>
              <span>Potential reclaim</span>
              <strong>{formatBytes(totalWaste)}</strong>
            </article>
          </section>

          {summary.warnings.length > 0 && (
            <details className="p6-warning-box">
              <summary>{summary.warnings.length} scan warnings</summary>
              <ul>
                {summary.warnings.slice(0, 20).map((warning, index) => (
                  <li key={`${warning.source}-${index}`}>
                    {warning.source}: {warning.message}
                  </li>
                ))}
              </ul>
            </details>
          )}

          <section>
            <div className="section-heading">
              <div>
                <p className="eyebrow">Verified duplicate groups</p>
                <h3>{summary.duplicateGroups.length} groups</h3>
              </div>
              <div className="p6-delete-summary">
                <span>{selectedIds.size} selected</span>
                <strong>{formatBytes(selectedBytes)}</strong>
                <button
                  className="danger-action"
                  disabled={selectedIds.size === 0}
                  onClick={() => setDeleteState({ status: "confirming" })}
                >
                  Delete selected…
                </button>
              </div>
            </div>

            {summary.duplicateGroups.length === 0 ? (
              <div className="empty-health-state">
                <strong>No byte-identical duplicates were proven.</strong>
                <span>
                  Files that match only by name, size, or partial hash are not
                  reported as duplicates.
                </span>
              </div>
            ) : (
              <div className="duplicate-group-list">
                {summary.duplicateGroups.map((group) => (
                  <article className="duplicate-group-card" key={group.id}>
                    <header>
                      <div>
                        <strong>{formatBytes(group.bytesEach)} each</strong>
                        <span>{group.files.length} verified copies</span>
                      </div>
                      <span className="status-badge neutral">
                        {formatBytes(group.recoverableBytes)} reclaimable
                      </span>
                    </header>
                    <div className="duplicate-file-list">
                      {group.files.map((file) => {
                        const checked = selectedIds.has(file.id);
                        const canSelect = canSelectDuplicate(
                          group,
                          selectedIds,
                          file.id,
                        );
                        return (
                          <label key={file.id}>
                            <input
                              checked={checked}
                              disabled={!checked && !canSelect}
                              onChange={() => toggleFile(group, file.id)}
                              type="checkbox"
                            />
                            <span>{file.path}</span>
                          </label>
                        );
                      })}
                    </div>
                    <small>At least one copy must remain in every group.</small>
                  </article>
                ))}
              </div>
            )}
          </section>
        </>
      )}

      {deleteState.status === "confirming" && summary && (
        <div className="startup-confirmation-backdrop" role="presentation">
          <section
            aria-labelledby="duplicate-delete-title"
            aria-modal="true"
            className="cleanup-confirmation startup-confirmation-dialog"
            role="dialog"
          >
            <p className="eyebrow">Destructive action</p>
            <h3 id="duplicate-delete-title">
              Delete {selectedIds.size} selected verified duplicate files?
            </h3>
            <p>
              Each file will be revalidated against its original root, size,
              timestamp, and full hash immediately before deletion. This action
              is not presented as restorable.
            </p>
            <div className="confirmation-actions">
              <button
                autoFocus
                className="secondary-action"
                onClick={() => setDeleteState({ status: "idle" })}
              >
                Cancel
              </button>
              <button
                className="danger-action"
                onClick={() => void executeDelete()}
              >
                Delete selected files
              </button>
            </div>
          </section>
        </div>
      )}

      {deleteState.status === "running" && (
        <section className="scan-progress">
          <span className="scan-spinner" aria-hidden="true" />
          <div>
            <strong>Revalidating selected duplicates…</strong>
            <span>Changed or unsafe files will be skipped, not forced.</span>
          </div>
        </section>
      )}
    </div>
  );
}
