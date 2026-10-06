import { useRef, useState } from "react";

import { cancelFilesystemScan, parseExclusions } from "./duplicates";
import {
  scanStorage,
  selectStorageRoot,
  type StorageScanSummary,
} from "./storage";
import type { ScanRootSelection } from "./duplicates";

type ScanState =
  | { status: "idle" }
  | { status: "scanning"; requestId: string }
  | { status: "ready"; summary: StorageScanSummary }
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

function newRequestId(): string {
  return `storage-${crypto.randomUUID()}`;
}

export function StoragePage() {
  const [roots, setRoots] = useState<ScanRootSelection[]>([]);
  const [exclusions, setExclusions] = useState(
    "node_modules, .git, target, .cache",
  );
  const [scanState, setScanState] = useState<ScanState>({ status: "idle" });
  const generation = useRef(0);

  const addRoot = async () => {
    try {
      const selected = await selectStorageRoot();
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
    const requestId = newRequestId();
    const currentGeneration = generation.current + 1;
    generation.current = currentGeneration;
    setScanState({ status: "scanning", requestId });

    try {
      const summary = await scanStorage(
        requestId,
        roots.map((root) => root.id),
        {
          excludedDirectoryNames: parseExclusions(exclusions),
          topFiles: 50,
          topFolders: 50,
        },
      );
      if (generation.current === currentGeneration) {
        setScanState({ status: "ready", summary });
      }
    } catch (error: unknown) {
      if (generation.current === currentGeneration) {
        const message = errorMessage(error, "Storage scan failed.");
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

  const summary = scanState.status === "ready" ? scanState.summary : null;

  return (
    <div className="page-stack p6-page">
      <section className="p6-hero">
        <div>
          <p className="eyebrow">Storage Analyzer — P6C</p>
          <h2>See where selected folders actually use disk space</h2>
          <p className="muted">
            Storage Analyzer is read-only. It aggregates real files without
            traversing links or Windows reparse points.
          </p>
        </div>
        <button className="secondary-action" onClick={() => void addRoot()}>
          Add folder
        </button>
      </section>

      <section className="p6-scan-config storage-config">
        <div>
          <strong>Selected folders</strong>
          {roots.length === 0 ? (
            <p className="muted">Choose at least one folder to analyze.</p>
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
            value={exclusions}
          />
        </label>
        <div className="p6-scan-actions">
          {scanState.status === "scanning" ? (
            <button
              className="secondary-action"
              onClick={() => void cancelScan()}
            >
              Cancel scan
            </button>
          ) : (
            <button
              className="primary-action"
              disabled={roots.length === 0}
              onClick={() => void startScan()}
            >
              Analyze storage
            </button>
          )}
        </div>
      </section>

      {scanState.status === "scanning" && (
        <section className="scan-progress">
          <span className="scan-spinner" aria-hidden="true" />
          <div>
            <strong>Aggregating real file sizes…</strong>
            <span>
              Inaccessible folders become warnings instead of stopping the scan.
            </span>
          </div>
        </section>
      )}

      {scanState.status === "error" && (
        <section className="info-callout health-error">
          <strong>Storage analysis failed</strong>
          <span>{scanState.message}</span>
        </section>
      )}

      {summary && (
        <>
          <section className="p6-summary-grid">
            <article>
              <span>Files scanned</span>
              <strong>{summary.totalFiles.toLocaleString()}</strong>
            </article>
            <article>
              <span>Total data</span>
              <strong>{formatBytes(summary.totalBytes)}</strong>
            </article>
            <article>
              <span>Folder aggregates</span>
              <strong>{summary.largestFolders.length}</strong>
            </article>
            <article>
              <span>File types</span>
              <strong>{summary.fileTypes.length}</strong>
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

          <section className="storage-columns">
            <article className="storage-table-card">
              <div className="section-heading">
                <div>
                  <p className="eyebrow">Largest folders</p>
                  <h3>Top folder aggregates</h3>
                </div>
              </div>
              <div className="storage-row-list">
                {summary.largestFolders.map((folder) => (
                  <div className="storage-row" key={folder.path}>
                    <span title={folder.path}>{folder.path}</span>
                    <strong>{formatBytes(folder.bytes)}</strong>
                    <small>{folder.fileCount.toLocaleString()} files</small>
                  </div>
                ))}
              </div>
            </article>

            <article className="storage-table-card">
              <div className="section-heading">
                <div>
                  <p className="eyebrow">Largest files</p>
                  <h3>Top files by size</h3>
                </div>
              </div>
              <div className="storage-row-list">
                {summary.largestFiles.map((file) => (
                  <div className="storage-row" key={file.path}>
                    <span title={file.path}>{file.path}</span>
                    <strong>{formatBytes(file.bytes)}</strong>
                  </div>
                ))}
              </div>
            </article>
          </section>

          <section className="storage-table-card">
            <div className="section-heading">
              <div>
                <p className="eyebrow">File types</p>
                <h3>Space by extension</h3>
              </div>
            </div>
            <div className="type-grid">
              {summary.fileTypes.map((type) => (
                <article key={type.extension}>
                  <strong>{type.extension}</strong>
                  <span>{formatBytes(type.bytes)}</span>
                  <small>{type.fileCount.toLocaleString()} files</small>
                </article>
              ))}
            </div>
          </section>
        </>
      )}
    </div>
  );
}
