# P7 EXECUTION PROMPT — System Monitor

Read `/AGENTS.md`, `/docs/MASTER_BUILD_PROMPT.md`, `/docs/IMPLEMENTATION_PLAN.md`, and `/docs/ARCHITECTURE.md` before changes.

## Goal

Build a low-overhead, honest Windows System Monitor that samples useful resource telemetry only while the Monitor page is active and unpaused.

P5 and P6 representative real-Windows manual verification remain deferred by explicit user instruction. Do not mark those manual gates complete without evidence.

## Required telemetry

Provide real data where Windows exposes a trustworthy source:

- total CPU utilization;
- RAM total/used/available and utilization;
- aggregate physical-disk read/write throughput where reliable;
- aggregate network receive/send throughput where reliable;
- top process consumers with PID, name, CPU and private working-set memory where available;
- GPU activity only when a supported Windows performance source is available;
- explicit sensor/capability status for unsupported telemetry such as temperature.

Never fabricate unavailable sensors. Use `Unavailable` with a short reason.

## Sampling model

- no permanent background monitor service in P7;
- UI polling exists only while the Monitor page is mounted and not paused;
- configurable interval with conservative choices (minimum 2 seconds);
- pause stops new native samples;
- changing interval must not create overlapping timers;
- native sampling must be bounded and return structured errors;
- stale/out-of-order results must not replace a newer sample;
- keep only a small bounded in-memory history for presentation.

## Provider

Prefer Windows performance/CIM sources that are stable enough for V1. A fixed PC Manager-authored PowerShell/CIM provider is acceptable in P7 when there is no arbitrary script input and its overhead is measured and surfaced.

The frontend must not provide PowerShell, WMI/CIM class names, commands, paths, or process IDs to the provider.

## Domain model

Suggested models:

```text
MonitorSnapshot
  collected_at_epoch_ms
  sample_duration_ms
  cpu
  memory
  disk
  network
  gpu
  top_processes
  sensors

MonitorAvailability
  available
  unavailable

MonitorError
  code
  message
  recoverable
```

Use optional values/status instead of fake zeroes when a source is unavailable.

## UI

Replace the Monitor placeholder with a functional page.

Required:

- CPU card;
- memory card;
- disk read/write card;
- network receive/send card;
- GPU card showing real supported metric or Unavailable;
- sensor/temperature capability card;
- top process table;
- pause/resume control;
- sampling interval selector;
- last-sample time and provider sample duration;
- loading/error states;
- bounded recent CPU/RAM history;
- responsive layout and no overflow.

## Overhead evidence

Instrument each native sample duration.

Document:

- selected polling intervals;
- no polling while paused;
- measured provider sample duration from Windows CI smoke evidence when available;
- any known cost/limitations of PowerShell/CIM acquisition.

Do not claim process-level idle CPU overhead was measured on a real user machine unless it actually was.

## Tests

Add tests for:

- availability/status serialization and parsing;
- percentage clamping/normalization;
- bounded history helper;
- polling interval normalization;
- pause scheduling helper;
- malformed provider payload handling where practical;
- Windows smoke sample returns without crashing and logs sample duration;
- unsupported GPU/sensor sources remain Unavailable rather than fabricated.

## Acceptance

P7 implementation is ready to merge when:

- real CPU and RAM sampling work on Windows CI;
- disk/network/top-process sources are represented honestly;
- GPU is real when supported or Unavailable;
- unsupported sensors show Unavailable;
- interval is configurable;
- pause stops polling;
- sample duration is surfaced;
- frontend/Rust/Windows CI are green;
- docs record limitations and deferred manual overhead verification.

Use branch `feat/p7-system-monitor`.
