use serde::Serialize;
use std::collections::BTreeMap;

use crate::{RiskLevel, StartupEntry, StartupSourceType};

const CPU_REVIEW_PERCENT: f64 = 5.0;
const CPU_MEDIUM_PERCENT: f64 = 20.0;
const MEMORY_REVIEW_BYTES: u64 = 512 * 1024 * 1024;
const MEMORY_MEDIUM_BYTES: u64 = 1024 * 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PerformanceFindingKind {
    PersistentProcess,
    StartupImpact,
    ScheduledTask,
    Service,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum PerformanceAction {
    ReviewStartup,
    ReviewAppSettings,
    ReviewScheduledTask,
    ReviewService,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceProcessSample {
    pub name: String,
    pub cpu_percent: Option<f64>,
    pub memory_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceSample {
    pub collected_at_epoch_ms: u64,
    pub cpu_percent: Option<f64>,
    pub memory_used_percent: Option<f64>,
    pub processes: Vec<PerformanceProcessSample>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceServiceEvidence {
    pub name: String,
    pub display_name: String,
    pub state: String,
    pub start_mode: String,
    pub process_name: Option<String>,
    pub windows_system_path: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceFinding {
    pub id: String,
    pub kind: PerformanceFindingKind,
    pub title: String,
    pub summary: String,
    pub evidence: Vec<String>,
    pub risk: RiskLevel,
    pub recommendation: String,
    pub action: PerformanceAction,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceOptimizerReport {
    pub generated_at_epoch_ms: u64,
    pub sample_count: usize,
    pub sample_window_ms: u64,
    pub average_cpu_percent: Option<f64>,
    pub average_memory_percent: Option<f64>,
    pub startup_entries_analyzed: usize,
    pub services_analyzed: usize,
    pub findings: Vec<PerformanceFinding>,
    pub limitations: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PerformanceOptimizerError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl PerformanceOptimizerError {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recoverable,
        }
    }
}

#[derive(Debug, Default)]
struct ProcessAggregate {
    display_name: String,
    appearances: usize,
    cpu_total: f64,
    cpu_samples: usize,
    max_memory_bytes: u64,
}

#[must_use]
pub fn evaluate_performance_optimizer(
    samples: &[PerformanceSample],
    startup_entries: &[StartupEntry],
    services: &[PerformanceServiceEvidence],
) -> PerformanceOptimizerReport {
    let generated_at_epoch_ms = samples
        .last()
        .map(|sample| sample.collected_at_epoch_ms)
        .unwrap_or_default();
    let sample_window_ms = samples
        .first()
        .zip(samples.last())
        .map(|(first, last)| {
            last.collected_at_epoch_ms
                .saturating_sub(first.collected_at_epoch_ms)
        })
        .unwrap_or_default();

    let average_cpu_percent =
        average_metric(samples.iter().filter_map(|sample| sample.cpu_percent));
    let average_memory_percent = average_metric(
        samples
            .iter()
            .filter_map(|sample| sample.memory_used_percent),
    );

    let mut aggregates: BTreeMap<String, ProcessAggregate> = BTreeMap::new();
    for sample in samples {
        for process in &sample.processes {
            let key = normalize_process_name(&process.name);
            if key.is_empty() || is_system_process(&key) {
                continue;
            }
            let aggregate = aggregates.entry(key).or_default();
            if aggregate.display_name.is_empty() {
                aggregate.display_name = process.name.trim().to_string();
            }
            aggregate.appearances += 1;
            if let Some(cpu) = process.cpu_percent.filter(|value| value.is_finite()) {
                aggregate.cpu_total += cpu.max(0.0);
                aggregate.cpu_samples += 1;
            }
            if let Some(memory) = process.memory_bytes {
                aggregate.max_memory_bytes = aggregate.max_memory_bytes.max(memory);
            }
        }
    }

    let minimum_appearances = if samples.len() >= 3 {
        2
    } else {
        samples.len().max(1)
    };
    let mut findings = Vec::new();

    for (process_key, aggregate) in aggregates {
        if aggregate.appearances < minimum_appearances {
            continue;
        }

        let average_process_cpu = if aggregate.cpu_samples > 0 {
            Some(aggregate.cpu_total / aggregate.cpu_samples as f64)
        } else {
            None
        };
        let cpu_evidence = average_process_cpu.unwrap_or_default() >= CPU_REVIEW_PERCENT;
        let memory_evidence = aggregate.max_memory_bytes >= MEMORY_REVIEW_BYTES;
        if !cpu_evidence && !memory_evidence {
            continue;
        }

        let risk = if average_process_cpu.unwrap_or_default() >= CPU_MEDIUM_PERCENT
            || aggregate.max_memory_bytes >= MEMORY_MEDIUM_BYTES
        {
            RiskLevel::Medium
        } else {
            RiskLevel::Low
        };

        let process_title = if aggregate.display_name.is_empty() {
            process_key.clone()
        } else {
            aggregate.display_name.clone()
        };
        let mut evidence = vec![format!(
            "Seen in {} of {} short samples.",
            aggregate.appearances,
            samples.len()
        )];
        if let Some(cpu) = average_process_cpu {
            evidence.push(format!("Average sampled CPU: {:.1}%.", cpu));
        }
        if aggregate.max_memory_bytes > 0 {
            evidence.push(format!(
                "Peak sampled private memory: {} MB.",
                aggregate.max_memory_bytes / (1024 * 1024)
            ));
        }

        findings.push(PerformanceFinding {
            id: format!("persistent-process-{}", findings.len() + 1),
            kind: PerformanceFindingKind::PersistentProcess,
            title: format!("{process_title} repeatedly used measurable resources"),
            summary: "This process stayed visible across multiple short samples with measurable CPU or memory use.".to_string(),
            evidence,
            risk,
            recommendation: "Review the application's own background/startup settings before considering any change. PC Manager does not suspend or kill it automatically.".to_string(),
            action: PerformanceAction::ReviewAppSettings,
        });

        for entry in startup_entries.iter().filter(|entry| entry.enabled) {
            if !startup_matches_process(entry, &process_key) {
                continue;
            }

            let (kind, action, label) = if entry.source_type == StartupSourceType::ScheduledTask {
                (
                    PerformanceFindingKind::ScheduledTask,
                    PerformanceAction::ReviewScheduledTask,
                    "scheduled task",
                )
            } else {
                (
                    PerformanceFindingKind::StartupImpact,
                    PerformanceAction::ReviewStartup,
                    "startup entry",
                )
            };

            findings.push(PerformanceFinding {
                id: format!("startup-evidence-{}", findings.len() + 1),
                kind,
                title: format!("{} is linked to an active {label}", entry.name),
                summary: "The startup source is not called slow merely because it exists; it is shown because its executable matches a repeatedly observed resource consumer.".to_string(),
                evidence: vec![
                    format!("Startup source: {}.", entry.source_label),
                    format!("Matched running process: {process_title}."),
                    format!("Change capability: {}.", if entry.can_change { "available in Startup Manager" } else { "review-only" }),
                ],
                risk: RiskLevel::Low,
                recommendation: if entry.can_change {
                    "Review this item in Startup Manager. If you disable it there, the existing rollback-aware startup workflow remains authoritative.".to_string()
                } else {
                    "Review the application's or Windows task settings. PC Manager will not elevate or disable this source from Performance Optimizer.".to_string()
                },
                action,
            });
        }

        for service in services {
            if service.windows_system_path {
                continue;
            }
            let Some(service_process) = service.process_name.as_deref() else {
                continue;
            };
            if normalize_process_name(service_process) != process_key {
                continue;
            }

            findings.push(PerformanceFinding {
                id: format!("service-evidence-{}", findings.len() + 1),
                kind: PerformanceFindingKind::Service,
                title: format!("{} runs automatically with {process_title}", service.display_name),
                summary: "This is an explainable review candidate because a non-Windows-path automatic service maps to a repeatedly observed resource consumer.".to_string(),
                evidence: vec![
                    format!("Service name: {}.", service.name),
                    format!("Service state: {}; start mode: {}.", service.state, service.start_mode),
                    format!("Matched running process: {process_title}."),
                ],
                risk: RiskLevel::Medium,
                recommendation: "Review the owning application's documentation before changing this service. P12 V1 deliberately provides no service-disable button.".to_string(),
                action: PerformanceAction::ReviewService,
            });
        }
    }

    PerformanceOptimizerReport {
        generated_at_epoch_ms,
        sample_count: samples.len(),
        sample_window_ms,
        average_cpu_percent,
        average_memory_percent,
        startup_entries_analyzed: startup_entries.len(),
        services_analyzed: services.len(),
        findings,
        limitations: vec![
            "This is a short foreground analysis, not a claim about long-term machine behavior.".to_string(),
            "P12 V1 does not implement a universal app-suspend/sleep mechanism because arbitrary suspension can break applications or lose work.".to_string(),
            "Performance Optimizer never disables services or scheduled tasks directly; supported startup changes remain owned by Startup Manager.".to_string(),
        ],
    }
}

fn average_metric(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut total = 0.0;
    let mut count = 0usize;
    for value in values.filter(|value| value.is_finite()) {
        total += value;
        count += 1;
    }
    (count > 0).then_some(total / count as f64)
}

fn normalize_process_name(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .trim_end_matches(".exe")
        .to_string()
}

fn startup_matches_process(entry: &StartupEntry, process_key: &str) -> bool {
    if process_key.is_empty() {
        return false;
    }
    let haystack = format!("{} {}", entry.name, entry.command).to_ascii_lowercase();
    haystack.contains(process_key)
}

fn is_system_process(process_key: &str) -> bool {
    matches!(
        process_key,
        "system" | "idle" | "registry" | "secure system" | "memory compression"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{StartupImpact, StartupSourceType};

    fn sample(at: u64, name: &str, cpu: f64, memory: u64) -> PerformanceSample {
        PerformanceSample {
            collected_at_epoch_ms: at,
            cpu_percent: Some(30.0),
            memory_used_percent: Some(60.0),
            processes: vec![PerformanceProcessSample {
                name: name.to_string(),
                cpu_percent: Some(cpu),
                memory_bytes: Some(memory),
            }],
        }
    }

    fn startup(command: &str, source_type: StartupSourceType) -> StartupEntry {
        StartupEntry {
            id: "entry-1".to_string(),
            name: "Example App".to_string(),
            source_type,
            source_label: "Current user Run".to_string(),
            command: command.to_string(),
            publisher: None,
            enabled: true,
            can_change: true,
            requires_elevation: false,
            impact: StartupImpact::Unknown,
            impact_evidence: None,
            detail: "test".to_string(),
        }
    }

    #[test]
    fn one_spike_is_not_called_persistent() {
        let report = evaluate_performance_optimizer(
            &[
                sample(1, "Example", 30.0, 800 * 1024 * 1024),
                PerformanceSample {
                    collected_at_epoch_ms: 2,
                    cpu_percent: Some(5.0),
                    memory_used_percent: Some(50.0),
                    processes: vec![],
                },
                PerformanceSample {
                    collected_at_epoch_ms: 3,
                    cpu_percent: Some(5.0),
                    memory_used_percent: Some(50.0),
                    processes: vec![],
                },
            ],
            &[],
            &[],
        );
        assert!(report.findings.is_empty());
    }

    #[test]
    fn persistent_process_can_link_to_startup_without_auto_disabling_it() {
        let report = evaluate_performance_optimizer(
            &[
                sample(100, "Example.exe", 12.0, 700 * 1024 * 1024),
                sample(900, "Example", 8.0, 650 * 1024 * 1024),
                sample(1700, "Example", 7.0, 620 * 1024 * 1024),
            ],
            &[startup(
                r#""C:\Program Files\Example\Example.exe" --background"#,
                StartupSourceType::RegistryCurrentUserRun,
            )],
            &[],
        );

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.kind == PerformanceFindingKind::PersistentProcess));
        assert!(report
            .findings
            .iter()
            .any(|finding| finding.kind == PerformanceFindingKind::StartupImpact));
        assert!(!report.findings.iter().any(|finding| finding
            .recommendation
            .to_ascii_lowercase()
            .contains("disable everything")));
    }

    #[test]
    fn windows_system_path_services_are_not_recommended() {
        let service = PerformanceServiceEvidence {
            name: "SystemLike".to_string(),
            display_name: "System Like".to_string(),
            state: "Running".to_string(),
            start_mode: "Auto".to_string(),
            process_name: Some("Example".to_string()),
            windows_system_path: true,
        };
        let report = evaluate_performance_optimizer(
            &[
                sample(1, "Example", 25.0, 900 * 1024 * 1024),
                sample(2, "Example", 25.0, 900 * 1024 * 1024),
            ],
            &[],
            &[service],
        );
        assert!(!report
            .findings
            .iter()
            .any(|finding| finding.kind == PerformanceFindingKind::Service));
    }
}
