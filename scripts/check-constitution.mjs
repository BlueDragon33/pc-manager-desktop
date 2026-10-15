import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const REQUIRED_PILLARS = [
  "structural-capacity",
  "architectural-longevity",
  "product-elegance",
  "premium-usability",
  "long-term-durability",
  "fortress-security-disaster-resilience",
  "operational-sovereignty-dependency-minimization"
];

const FORBIDDEN_REMOTE_OPERATIONS = [
  "cloud-required-privileged-operation",
  "remote-generic-shell",
  "remote-powershell",
  "remote-arbitrary-process-execution",
  "remote-arbitrary-registry-write",
  "cloud-owned-cleanup-or-restore-authority",
  "plaintext-sensitive-system-data-in-drive-or-sheets"
];

/**
 * Local boundary check. The central reusable Constitution workflow independently
 * verifies policyVersion against Software-Blueprint-Hub's latest canonical contract.
 * Never substitute local version agreement for canonical version validation.
 */
export function validateGovernance(adoption, budget) {
  const errors = [];
  const requireRule = (ok, reason) => {
    if (!ok) errors.push(reason);
  };

  const version = adoption?.policyVersion;
  requireRule(
    typeof version === "string" && /^\d+\.\d+\.\d+$/.test(version),
    "adoption.policyVersion must be a semantic version"
  );
  requireRule(
    version === budget?.policyVersion,
    "dependency budget policyVersion must match adoption.policyVersion"
  );
  requireRule(
    adoption?.policyId === "blueprint-os:universal-century-grade" &&
      budget?.policyId === adoption.policyId,
    "project and dependency budget must retain the canonical policyId"
  );
  requireRule(
    adoption?.projectId === "project:pc-manager-desktop" &&
      budget?.projectId === adoption.projectId,
    "projectId mismatch"
  );
  requireRule(adoption?.enforcementMode === "enforced", "enforcement must remain enabled");
  requireRule(
    adoption?.productionAuthority === "separate-explicit-release-gate",
    "production requires separate explicit release authority"
  );
  requireRule(
    Array.isArray(adoption?.disabledPillars) && adoption.disabledPillars.length === 0,
    "constitutional pillars may not be disabled"
  );
  requireRule(
    Array.isArray(adoption?.constitutionalWaivers) &&
      adoption.constitutionalWaivers.length === 0,
    "unreviewed constitutional waivers are forbidden"
  );

  const pillars = new Set(Array.isArray(adoption?.inheritedPillars) ? adoption.inheritedPillars : []);
  for (const pillar of REQUIRED_PILLARS) {
    requireRule(pillars.has(pillar), "missing mandatory pillar: " + pillar);
  }

  requireRule(budget?.posture === "LOCAL_CORE", "PC Manager must remain LOCAL_CORE");
  requireRule(
    budget?.canonicalAuthority?.privilegedActions === "local-tauri-rust-and-narrow-windows-service-ipc" &&
      budget?.canonicalAuthority?.systemState === "local-machine" &&
      budget?.canonicalAuthority?.remoteManagement === "optional-typed-coordination-only",
    "machine authority must stay local with typed optional remote coordination"
  );

  const deps = Array.isArray(budget?.dependencies) ? budget.dependencies : [];
  requireRule(Array.isArray(budget?.dependencies), "dependencies must be an array");
  const ids = deps.map((dep) => dep?.id);
  requireRule(new Set(ids).size === ids.length && ids.every(Boolean), "duplicate or missing dependency id");
  const windows = deps.find((dep) => dep?.id === "windows-host");
  const manager = deps.find((dep) => dep?.id === "application-management");
  requireRule(windows?.class === "EXTERNAL_ESSENTIAL", "Windows host must remain essential");
  requireRule(manager?.class === "OPTIONAL_SYNC", "Application Management must remain optional");
  for (const dep of deps) {
    requireRule(
      typeof dep?.offlineBehavior === "string" && dep.offlineBehavior.trim().length > 0 &&
        typeof dep?.exitPath === "string" && dep.exitPath.trim().length > 0,
      "every dependency must have offlineBehavior and exitPath: " + String(dep?.id)
    );
  }

  const forbidden = new Set(Array.isArray(budget?.forbidden) ? budget.forbidden : []);
  for (const rule of FORBIDDEN_REMOTE_OPERATIONS) {
    requireRule(forbidden.has(rule), "missing privileged boundary rule: " + rule);
  }
  requireRule(
    typeof budget?.releaseGate === "string" &&
      budget.releaseGate.includes("Production remains separately authorized"),
    "production release authority must remain independent"
  );
  return errors;
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const adoption = JSON.parse(fs.readFileSync(".blueprint/constitution-adoption.json", "utf8"));
    const budget = JSON.parse(fs.readFileSync(".blueprint/dependency-budget.json", "utf8"));
    const errors = validateGovernance(adoption, budget);
    for (const error of errors) console.error("[constitution-local] " + error);
    if (errors.length) process.exitCode = 1;
    else console.log("[constitution-local] PASS local authority and dependency sovereignty");
  } catch (error) {
    console.error("[constitution-local] fail closed: " + String(error));
    process.exitCode = 1;
  }
}
