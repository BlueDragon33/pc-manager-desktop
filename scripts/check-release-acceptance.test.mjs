import assert from "node:assert/strict";
import { test } from "node:test";

import {
  REQUIRED_GATES,
  evaluateReleaseAcceptance,
} from "./check-release-acceptance.mjs";

function manifest() {
  return {
    schema: "pc-manager.release-acceptance/v1",
    gates: REQUIRED_GATES.map((id) => ({
      id,
      description: `Manual acceptance ${id}`,
      status: "pending",
      verifiedAt: null,
      evidence: null,
    })),
  };
}

test("pending manual gates are schema-valid but NEVER release-ready", () => {
  const result = evaluateReleaseAcceptance(manifest());
  assert.equal(result.valid, true);
  assert.equal(result.ready, false);
  assert.equal(result.pending.length, REQUIRED_GATES.length);
});

test("missing and duplicate gates fail closed", () => {
  const missing = manifest();
  missing.gates.pop();
  assert.equal(evaluateReleaseAcceptance(missing).valid, false);
  const duplicate = manifest();
  duplicate.gates.push(duplicate.gates[0]);
  assert.equal(evaluateReleaseAcceptance(duplicate).valid, false);
});

test("self-declared acceptance without verifiable metadata is rejected", () => {
  const sample = manifest();
  sample.gates[0].status = "accepted";
  assert.equal(evaluateReleaseAcceptance(sample).valid, false);
  sample.gates[0].verifiedAt = "2026-10-08T12:00:00Z";
  sample.gates[0].evidence = "unverified screenshot";
  assert.equal(evaluateReleaseAcceptance(sample).valid, false);
});

test("complete acceptance requires every gate to carry a dated HTTPS record", () => {
  const sample = manifest();
  sample.gates.forEach((gate) => {
    gate.status = "accepted";
    gate.verifiedAt = "2026-10-08T12:00:00Z";
    gate.evidence = "https://github.com/BlueDragon33/pc-manager-desktop/issues/123";
  });
  const result = evaluateReleaseAcceptance(sample);
  assert.equal(result.valid, true);
  assert.equal(result.ready, true);
});
