import assert from "node:assert/strict";
import fs from "node:fs";
import test from "node:test";
import { validateGovernance } from "./check-constitution.mjs";

const read = (name) => JSON.parse(fs.readFileSync(name, "utf8"));
const fresh = () => [
  read(".blueprint/constitution-adoption.json"),
  read(".blueprint/dependency-budget.json")
];

test("current local governance matches the adoption manifest", () => {
  assert.deepEqual(validateGovernance(...fresh()), []);
});

test("rejects stale or altered dependency budget policy version", () => {
  const [adoption, budget] = fresh();
  budget.policyVersion = "1.2.0";
  assert.match(validateGovernance(adoption, budget).join("\n"), /policyVersion/);
});

test("rejects removal of mandatory sovereignty pillar", () => {
  const [adoption, budget] = fresh();
  adoption.inheritedPillars = adoption.inheritedPillars.filter(
    (name) => name !== "operational-sovereignty-dependency-minimization"
  );
  assert.match(validateGovernance(adoption, budget).join("\n"), /mandatory pillar/);
});

test("rejects cloud-owned machine authority and shell permissions", () => {
  const [adoption, budget] = fresh();
  budget.canonicalAuthority.privilegedActions = "remote-shell";
  budget.forbidden = budget.forbidden.filter((rule) => rule !== "remote-generic-shell");
  const errors = validateGovernance(adoption, budget).join("\n");
  assert.match(errors, /machine authority/);
  assert.match(errors, /remote-generic-shell/);
});

test("rejects compulsory App Manager and absent offline fallback", () => {
  const [adoption, budget] = fresh();
  const manager = budget.dependencies.find((dep) => dep.id === "application-management");
  manager.class = "EXTERNAL_ESSENTIAL";
  manager.offlineBehavior = "";
  const errors = validateGovernance(adoption, budget).join("\n");
  assert.match(errors, /must remain optional/);
  assert.match(errors, /offlineBehavior/);
});

test("rejects an implicit production release or disabled enforcement", () => {
  const [adoption, budget] = fresh();
  adoption.enforcementMode = "optional";
  adoption.productionAuthority = "automatic";
  assert.match(validateGovernance(adoption, budget).join("\n"), /enforcement/);
  assert.match(validateGovernance(adoption, budget).join("\n"), /release authority/);
});

test("rejects waiver, duplicate dependency or missing release statement", () => {
  const [adoption, budget] = fresh();
  adoption.constitutionalWaivers.push("skip-security");
  budget.dependencies.push({ ...budget.dependencies[0] });
  budget.releaseGate = "CI only";
  const errors = validateGovernance(adoption, budget).join("\n");
  assert.match(errors, /waivers/);
  assert.match(errors, /duplicate/);
  assert.match(errors, /independent/);
});
