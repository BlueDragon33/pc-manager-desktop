import fs from "node:fs";

function fail(message) {
  console.error("[constitution-1.2] " + message);
  process.exitCode = 1;
}

const adoption = JSON.parse(
  fs.readFileSync(".blueprint/constitution-adoption.json", "utf8")
);
const budget = JSON.parse(
  fs.readFileSync(".blueprint/dependency-budget.json", "utf8")
);

if (adoption.policyVersion !== "1.2.0") fail("policyVersion must be 1.2.0");
if (!adoption.inheritedPillars.includes("operational-sovereignty-dependency-minimization")) {
  fail("operational sovereignty pillar missing");
}
if (budget.policyVersion !== "1.2.0") fail("dependency budget policy mismatch");
if (budget.projectId !== adoption.projectId) fail("dependency budget projectId mismatch");
if (budget.posture !== "LOCAL_CORE") fail("PC Manager must remain LOCAL_CORE");

const forbidden = new Set(budget.forbidden ?? []);
for (const rule of [
  "cloud-required-privileged-operation",
  "remote-generic-shell",
  "remote-powershell",
  "remote-arbitrary-process-execution",
  "remote-arbitrary-registry-write",
  "cloud-owned-cleanup-or-restore-authority"
]) {
  if (!forbidden.has(rule)) fail("missing privileged-boundary rule: " + rule);
}

const appManager = (budget.dependencies ?? []).find((d) => d.id === "application-management");
if (!appManager || appManager.class !== "OPTIONAL_SYNC") {
  fail("Application Management must stay optional for local privileged operation");
}

if (!process.exitCode) {
  console.log("[constitution-1.2] PASS PC Manager local privileged authority + dependency budget");
}
