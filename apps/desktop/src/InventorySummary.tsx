import { useEffect, useState } from "react";

import {
  formatBytes,
  getSystemInventory,
  type InventoryError,
  type SystemInventory,
} from "./systemInventory";

type InventoryState =
  | { status: "loading" }
  | { status: "ready"; inventory: SystemInventory }
  | { status: "error"; message: string; recoverable: boolean };

function describeError(error: unknown): {
  message: string;
  recoverable: boolean;
} {
  if (typeof error === "object" && error !== null) {
    const candidate = error as Partial<InventoryError>;
    if (typeof candidate.message === "string") {
      return {
        message: candidate.message,
        recoverable: candidate.recoverable ?? true,
      };
    }
  }

  return {
    message: error instanceof Error ? error.message : "Inventory is unavailable.",
    recoverable: true,
  };
}

export function InventorySummary() {
  const [state, setState] = useState<InventoryState>({ status: "loading" });

  const loadInventory = () => {
    setState({ status: "loading" });
    getSystemInventory()
      .then((inventory) => setState({ status: "ready", inventory }))
      .catch((error: unknown) => {
        const described = describeError(error);
        setState({
          status: "error",
          message: described.message,
          recoverable: described.recoverable,
        });
      });
  };

  useEffect(() => {
    let active = true;

    getSystemInventory()
      .then((inventory) => {
        if (active) {
          setState({ status: "ready", inventory });
        }
      })
      .catch((error: unknown) => {
        if (active) {
          const described = describeError(error);
          setState({
            status: "error",
            message: described.message,
            recoverable: described.recoverable,
          });
        }
      });

    return () => {
      active = false;
    };
  }, []);

  if (state.status === "loading") {
    return (
      <section className="inventory-card" aria-live="polite">
        <div>
          <p className="eyebrow">Device snapshot</p>
          <h3>Reading Windows inventory…</h3>
        </div>
        <span className="status-badge neutral">Read-only</span>
      </section>
    );
  }

  if (state.status === "error") {
    return (
      <section className="inventory-card inventory-error" aria-live="polite">
        <div>
          <p className="eyebrow">Device snapshot</p>
          <h3>Windows inventory unavailable</h3>
          <p className="muted">{state.message}</p>
        </div>
        {state.recoverable && (
          <button className="secondary-action" onClick={loadInventory}>
            Retry
          </button>
        )}
      </section>
    );
  }

  const { inventory } = state;
  const volumeTotal = inventory.volumes.reduce(
    (sum, volume) => sum + volume.totalBytes,
    0,
  );
  const volumeAvailable = inventory.volumes.reduce(
    (sum, volume) => sum + volume.availableBytes,
    0,
  );

  return (
    <section className="inventory-card" aria-labelledby="device-snapshot-heading">
      <div className="inventory-header">
        <div>
          <p className="eyebrow">Device snapshot</p>
          <h3 id="device-snapshot-heading">{inventory.device.hostname}</h3>
          <p className="muted">
            Real, read-only Windows inventory. Nothing is changed by this view.
          </p>
        </div>
        <span className="status-badge good">Inventory ready</span>
      </div>

      <div className="inventory-grid">
        <article>
          <span>Operating system</span>
          <strong>{inventory.operatingSystem.name}</strong>
          <small>
            {inventory.operatingSystem.version}
            {inventory.operatingSystem.buildNumber
              ? ` · build ${inventory.operatingSystem.buildNumber}`
              : ""}
          </small>
        </article>
        <article>
          <span>Processor</span>
          <strong>{inventory.cpu.brand}</strong>
          <small>
            {inventory.cpu.physicalCores ?? "?"} physical /{" "}
            {inventory.cpu.logicalCores || "?"} logical cores
          </small>
        </article>
        <article>
          <span>Memory</span>
          <strong>{formatBytes(inventory.memory.totalBytes)}</strong>
          <small>{formatBytes(inventory.memory.availableBytes)} available</small>
        </article>
        <article>
          <span>Local storage</span>
          <strong>{formatBytes(volumeTotal)}</strong>
          <small>
            {formatBytes(volumeAvailable)} available · {inventory.volumes.length}{" "}
            volume(s)
          </small>
        </article>
        <article>
          <span>Processes</span>
          <strong>{inventory.processes.totalCount}</strong>
          <small>Read-only process inventory</small>
        </article>
        <article>
          <span>Installed apps</span>
          <strong>{inventory.installedApplications.length}</strong>
          <small>Registry inventory, deduplicated</small>
        </article>
        <article>
          <span>Startup items</span>
          <strong>{inventory.startupItems.length}</strong>
          <small>No startup state has been changed</small>
        </article>
        <article>
          <span>Network adapters</span>
          <strong>{inventory.networkAdapters.length}</strong>
          <small>
            {inventory.warnings.length
              ? `${inventory.warnings.length} source warning(s)`
              : "All requested sources returned"}
          </small>
        </article>
      </div>
    </section>
  );
}
