#!/usr/bin/env node
/**
 * Release acceptance is a human-evidence gate, separate from green CI.
 * --validate checks manifest structure without requiring acceptance.
 * --require-ready fails closed before ANY signed production packaging.
 */
import { readFileSync } from "node:fs";

export const REQUIRED_GATES = Object.freeze([
  "P5_STARTUP_ROUNDTRIP",
  "P6_USER_FILE_OPERATIONS",
  "P7_IDLE_OVERHEAD",
  "P8_SIGNED_GATEWAY_HANDSHAKE",
  "P9_SIGNED_CLEAN_INSTALL",
  "P9_SIGNED_UPGRADE_PRESERVATION",
  "P10_WINGET_INTERACTIVE_UPDATE",
  "P11_DRIVER_OPTIONAL_UPDATES",
  "P12_PERFORMANCE_EVIDENCE",
]);

export function evaluateReleaseAcceptance(document) {
  const errors = [];
  const pending = [];
  if (!document || typeof document !== "object" || Array.isArray(document)) {
    return { valid: false, ready: false, errors: ["Manifest must be an object."], pending };
  }
  if (document.schema !== "pc-manager.release-acceptance/v1") {
    errors.push("Unsupported acceptance schema.");
  }
  if (!Array.isArray(document.gates)) {
    return { valid: false, ready: false, errors: [...errors, "gates must be an array."], pending };
  }
  const required = new Set(REQUIRED_GATES);
  const seen = new Set();
  for (const gate of document.gates) {
    if (!gate || typeof gate !== "object" || Array.isArray(gate)) {
      errors.push("Every gate must be an object.");
      continue;
    }
    if (typeof gate.id !== "string" || !required.has(gate.id)) {
      errors.push("Unknown release gate identifier.");
      continue;
    }
    if (seen.has(gate.id)) {
      errors.push(`Duplicate release gate: ${gate.id}`);
      continue;
    }
    seen.add(gate.id);
    if (typeof gate.description !== "string" || !gate.description.trim()) {
      errors.push(`Missing gate description: ${gate.id}`);
    }
    if (gate.status === "pending") {
      pending.push(gate.id);
      if (gate.verifiedAt !== null || gate.evidence !== null) {
        errors.push(`Pending gate cannot assert evidence: ${gate.id}`);
      }
    } else if (gate.status === "accepted") {
      const verifiedAt = gate.verifiedAt;
      const evidence = gate.evidence;
      if (
        typeof verifiedAt !== "string" ||
        !/^\\d{4}-\\d{2}-\\d{2}T\\d{2}:\\d{2}:\\d{2}/.test(verifiedAt) ||
        !Number.isFinite(Date.parse(verifiedAt)) ||
        typeof evidence !== "string" ||
        !/^https:\\/\\//.test(evidence)
      ) {
        errors.push(`Accepted gate requires ISO verification time and HTTPS evidence: ${gate.id}`);
      }
    } else {
      errors.push(`Invalid gate status: ${gate.id}`);
    }
  }
  for (const id of required) {
    if (!seen.has(id)) {
      errors.push(`Missing mandatory release gate: ${id}`);
    }
  }
  return {
    valid: errors.length === 0,
    ready: errors.length === 0 && pending.length === 0,
    errors,
    pending,
  };
}

function main() {
  const mode = process.argv[2] ?? "--validate";
  if (mode !== "--validate" && mode !== "--require-ready") {
    console.error("Usage: node scripts/check-release-acceptance.mjs [--validate|--require-ready]");
    process.exitCode = 2;
    return;
  }
  let manifest;
  try {
    manifest = JSON.parse(readFileSync(new URL("../docs/release-acceptance.json", import.meta.url), "utf8"));
  } catch (error) {
    console.error(`Unable to load release evidence: ${error.message}`);
    process.exitCode = 1;
    return;
  }
  const result = evaluateReleaseAcceptance(manifest);
  if (!result.valid) {
    for (const error of result.errors) console.error(`[release-acceptance] FAIL ${error}`);
    process.exitCode = 1;
    return;
  }
  if (mode === "--require-ready" && !result.ready) {
    console.error(`[release-acceptance] BLOCKED: ${result.pending.join(", ")}`);
    process.exitCode = 1;
    return;
  }
  console.log(`[release-acceptance] ${mode === "--validate" ? "SCHEMA PASS" : "ALL GATES ACCEPTED"}; pending=${result.pending.length}`);
}

if (process.argv[1] && import.meta.url === new URL(`file://${process.argv[1]}`).href) {
  main();
}
