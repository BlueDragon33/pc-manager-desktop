import { invoke } from "@tauri-apps/api/core";

export interface SoftwareUpdateCandidate {
  id: string;
  name: string;
  packageId: string;
  installedVersion: string;
  availableVersion: string;
  source: string;
  sourceVerified: boolean;
  publisher: string | null;
  publisherVerification: string;
  detail: string;
}

export interface SoftwareUpdateScan {
  scanId: string;
  provider: string;
  providerAvailable: boolean;
  sourceVerified: boolean;
  sourceDetail: string;
  candidates: SoftwareUpdateCandidate[];
  warnings: string[];
}

export interface SoftwareUpdateLaunchResult {
  candidateId: string;
  packageId: string;
  displayName: string;
  launched: boolean;
  message: string;
}

export async function checkSoftwareUpdates(): Promise<SoftwareUpdateScan> {
  return invoke<SoftwareUpdateScan>("check_software_updates");
}

export async function launchSoftwareUpdate(
  scanId: string,
  candidateId: string,
): Promise<SoftwareUpdateLaunchResult> {
  return invoke<SoftwareUpdateLaunchResult>("launch_software_update", {
    scanId,
    candidateId,
  });
}

export function filterUpdateCandidates(
  candidates: SoftwareUpdateCandidate[],
  query: string,
): SoftwareUpdateCandidate[] {
  const needle = query.trim().toLocaleLowerCase();
  if (!needle) return candidates;

  return candidates.filter((candidate) =>
    [
      candidate.name,
      candidate.packageId,
      candidate.publisher ?? "",
      candidate.installedVersion,
      candidate.availableVersion,
      candidate.source,
    ]
      .join(" ")
      .toLocaleLowerCase()
      .includes(needle),
  );
}
