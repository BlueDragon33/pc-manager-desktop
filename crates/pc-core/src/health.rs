use serde::Serialize;

use crate::{InventoryWarning, SystemInventory};

const TOTAL_CATEGORIES: u8 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HealthCategory {
    Storage,
    Performance,
    Security,
    Updates,
    Privacy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HealthStatus {
    Good,
    Attention,
    Critical,
    Unavailable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum HealthSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum RiskLevel {
    None,
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthFinding {
    pub id: String,
    pub category: HealthCategory,
    pub title: String,
    pub description: String,
    pub evidence: String,
    pub severity: HealthSeverity,
    pub risk: RiskLevel,
    pub recommended_action: String,
    pub action_available: bool,
    pub reversible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthCategoryResult {
    pub category: HealthCategory,
    pub status: HealthStatus,
    pub score: Option<u8>,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HealthReport {
    pub scan_id: String,
    pub collected_at_epoch_ms: u64,
    pub score: Option<u8>,
    pub supported_categories: u8,
    pub total_categories: u8,
    pub coverage_percent: u8,
    pub category_results: Vec<HealthCategoryResult>,
    pub findings: Vec<HealthFinding>,
    pub inventory_warnings: Vec<InventoryWarning>,
}

#[must_use]
pub fn evaluate_health(inventory: &SystemInventory) -> HealthReport {
    let mut findings = Vec::new();
    let mut categories = Vec::with_capacity(TOTAL_CATEGORIES as usize);

    categories.push(evaluate_storage(inventory, &mut findings));
    categories.push(evaluate_performance(inventory, &mut findings));
    categories.push(unavailable(
        HealthCategory::Security,
        "Security checks are not implemented in Health Check V1.",
    ));
    categories.push(unavailable(
        HealthCategory::Updates,
        "Update checks are not implemented in Health Check V1.",
    ));
    categories.push(unavailable(
        HealthCategory::Privacy,
        "Privacy checks are not implemented in Health Check V1.",
    ));

    let supported_scores = categories
        .iter()
        .filter_map(|category| category.score)
        .collect::<Vec<_>>();

    let supported_categories = u8::try_from(supported_scores.len()).unwrap_or(TOTAL_CATEGORIES);
    let score = if supported_scores.is_empty() {
        None
    } else {
        let total: u16 = supported_scores.iter().map(|value| u16::from(*value)).sum();
        Some((total / supported_scores.len() as u16) as u8)
    };
    let coverage_percent = supported_categories.saturating_mul(100) / TOTAL_CATEGORIES;

    HealthReport {
        scan_id: scan_id(inventory),
        collected_at_epoch_ms: inventory.collected_at_epoch_ms,
        score,
        supported_categories,
        total_categories: TOTAL_CATEGORIES,
        coverage_percent,
        category_results: categories,
        findings,
        inventory_warnings: inventory.warnings.clone(),
    }
}

fn evaluate_storage(
    inventory: &SystemInventory,
    findings: &mut Vec<HealthFinding>,
) -> HealthCategoryResult {
    let measurable = inventory
        .volumes
        .iter()
        .filter(|volume| volume.total_bytes > 0)
        .collect::<Vec<_>>();

    if measurable.is_empty() {
        return unavailable(
            HealthCategory::Storage,
            "No local fixed volume with measurable capacity was returned.",
        );
    }

    let mut status = HealthStatus::Good;
    let mut score = 100_u8;

    for volume in measurable {
        let free_percent = percent(volume.available_bytes, volume.total_bytes);

        if free_percent < 10 {
            status = HealthStatus::Critical;
            score = score.saturating_sub(40);
            findings.push(HealthFinding {
                id: format!("storage-low-space:{}", volume.name.to_ascii_lowercase()),
                category: HealthCategory::Storage,
                title: format!("Very little free space on {}", volume.name),
                description:
                    "The volume has less than 10% free capacity. Low free space can interfere with updates and temporary working files."
                        .to_string(),
                evidence: format!(
                    "{}% free ({} of {} bytes available)",
                    free_percent, volume.available_bytes, volume.total_bytes
                ),
                severity: HealthSeverity::Critical,
                risk: RiskLevel::Medium,
                recommended_action:
                    "Review large files and cleanup candidates before deciding what to remove."
                        .to_string(),
                action_available: false,
                reversible: false,
            });
        } else if free_percent < 20 {
            if status != HealthStatus::Critical {
                status = HealthStatus::Attention;
            }
            score = score.saturating_sub(20);
            findings.push(HealthFinding {
                id: format!(
                    "storage-space-attention:{}",
                    volume.name.to_ascii_lowercase()
                ),
                category: HealthCategory::Storage,
                title: format!("Free space is getting low on {}", volume.name),
                description:
                    "The volume has less than 20% free capacity. There is no immediate cleanup action in Health Check V1."
                        .to_string(),
                evidence: format!(
                    "{}% free ({} of {} bytes available)",
                    free_percent, volume.available_bytes, volume.total_bytes
                ),
                severity: HealthSeverity::Warning,
                risk: RiskLevel::Low,
                recommended_action:
                    "Review storage usage and remove only files or caches you understand."
                        .to_string(),
                action_available: false,
                reversible: false,
            });
        }
    }

    HealthCategoryResult {
        category: HealthCategory::Storage,
        status,
        score: Some(score),
        summary: match status {
            HealthStatus::Good => "Local storage has comfortable free capacity.".to_string(),
            HealthStatus::Attention => {
                "At least one local volume is below 20% free capacity.".to_string()
            }
            HealthStatus::Critical => {
                "At least one local volume is below 10% free capacity.".to_string()
            }
            HealthStatus::Unavailable => unreachable!("storage is supported in this branch"),
        },
    }
}

fn evaluate_performance(
    inventory: &SystemInventory,
    findings: &mut Vec<HealthFinding>,
) -> HealthCategoryResult {
    if inventory.memory.total_bytes == 0 {
        return unavailable(
            HealthCategory::Performance,
            "Memory capacity was unavailable, so performance pressure cannot be scored.",
        );
    }

    let available_percent = percent(
        inventory.memory.available_bytes,
        inventory.memory.total_bytes,
    );

    if available_percent < 10 {
        findings.push(HealthFinding {
            id: "performance-memory-pressure".to_string(),
            category: HealthCategory::Performance,
            title: "Available memory is very low".to_string(),
            description:
                "Less than 10% of physical memory was available when the inventory snapshot was collected."
                    .to_string(),
            evidence: format!(
                "{}% available ({} of {} bytes)",
                available_percent, inventory.memory.available_bytes, inventory.memory.total_bytes
            ),
            severity: HealthSeverity::Critical,
            risk: RiskLevel::Medium,
            recommended_action:
                "Review the applications using the most memory before closing anything."
                    .to_string(),
            action_available: false,
            reversible: false,
        });

        HealthCategoryResult {
            category: HealthCategory::Performance,
            status: HealthStatus::Critical,
            score: Some(60),
            summary: "The current snapshot shows high memory pressure.".to_string(),
        }
    } else if available_percent < 20 {
        findings.push(HealthFinding {
            id: "performance-memory-pressure".to_string(),
            category: HealthCategory::Performance,
            title: "Available memory is getting low".to_string(),
            description:
                "Less than 20% of physical memory was available when the inventory snapshot was collected."
                    .to_string(),
            evidence: format!(
                "{}% available ({} of {} bytes)",
                available_percent, inventory.memory.available_bytes, inventory.memory.total_bytes
            ),
            severity: HealthSeverity::Warning,
            risk: RiskLevel::Low,
            recommended_action:
                "Review the applications using the most memory if the PC feels slow."
                    .to_string(),
            action_available: false,
            reversible: false,
        });

        HealthCategoryResult {
            category: HealthCategory::Performance,
            status: HealthStatus::Attention,
            score: Some(80),
            summary: "The current snapshot shows moderate memory pressure.".to_string(),
        }
    } else {
        HealthCategoryResult {
            category: HealthCategory::Performance,
            status: HealthStatus::Good,
            score: Some(100),
            summary: "The memory snapshot does not show significant pressure.".to_string(),
        }
    }
}

fn unavailable(category: HealthCategory, summary: &str) -> HealthCategoryResult {
    HealthCategoryResult {
        category,
        status: HealthStatus::Unavailable,
        score: None,
        summary: summary.to_string(),
    }
}

fn percent(part: u64, whole: u64) -> u8 {
    if whole == 0 {
        return 0;
    }

    let value = (u128::from(part).saturating_mul(100) / u128::from(whole)).min(100);
    value as u8
}

fn scan_id(inventory: &SystemInventory) -> String {
    let hostname = inventory
        .device
        .hostname
        .chars()
        .filter(|character| character.is_ascii_alphanumeric() || *character == '-')
        .collect::<String>()
        .to_ascii_lowercase();

    format!("health-{}-{hostname}", inventory.collected_at_epoch_ms)
}

#[cfg(test)]
mod tests {
    use super::{
        evaluate_health, HealthCategory, HealthSeverity, HealthStatus, TOTAL_CATEGORIES,
    };
    use crate::{
        CpuSummary, DeviceIdentity, DiskVolume, MemorySummary, OperatingSystemSummary,
        ProcessSummary, SystemInventory,
    };

    fn inventory(free_disk_percent: u64, available_memory_percent: u64) -> SystemInventory {
        SystemInventory {
            collected_at_epoch_ms: 1_700_000_000_000,
            operating_system: OperatingSystemSummary {
                name: "Windows 11".to_string(),
                version: "10.0".to_string(),
                build_number: Some("26000".to_string()),
                architecture: "x64".to_string(),
            },
            device: DeviceIdentity {
                hostname: "TEST-PC".to_string(),
                local_device_id: Some("abc".to_string()),
            },
            cpu: CpuSummary {
                brand: "CPU".to_string(),
                physical_cores: Some(4),
                logical_cores: 8,
                architecture: "x64".to_string(),
            },
            memory: MemorySummary {
                total_bytes: 100,
                available_bytes: available_memory_percent,
            },
            volumes: vec![DiskVolume {
                name: "C:".to_string(),
                label: None,
                file_system: Some("NTFS".to_string()),
                total_bytes: 100,
                available_bytes: free_disk_percent,
            }],
            processes: ProcessSummary {
                total_count: 0,
                top_memory: vec![],
            },
            installed_applications: vec![],
            startup_items: vec![],
            network_adapters: vec![],
            warnings: vec![],
        }
    }

    #[test]
    fn healthy_supported_categories_score_without_penalizing_unavailable_checks() {
        let report = evaluate_health(&inventory(50, 50));

        assert_eq!(report.supported_categories, 2);
        assert_eq!(report.total_categories, TOTAL_CATEGORIES);
        assert_eq!(report.coverage_percent, 40);
        assert_eq!(report.score, Some(100));
        assert_eq!(
            report
                .category_results
                .iter()
                .filter(|result| result.status == HealthStatus::Unavailable)
                .count(),
            3
        );
    }

    #[test]
    fn storage_thresholds_are_deterministic() {
        let warning = evaluate_health(&inventory(19, 50));
        let critical = evaluate_health(&inventory(9, 50));

        assert!(warning.findings.iter().any(|finding| {
            finding.category == HealthCategory::Storage
                && finding.severity == HealthSeverity::Warning
        }));
        assert!(critical.findings.iter().any(|finding| {
            finding.category == HealthCategory::Storage
                && finding.severity == HealthSeverity::Critical
        }));
    }

    #[test]
    fn memory_thresholds_are_deterministic() {
        let warning = evaluate_health(&inventory(50, 19));
        let critical = evaluate_health(&inventory(50, 9));

        assert_eq!(
            warning
                .category_results
                .iter()
                .find(|result| result.category == HealthCategory::Performance)
                .expect("performance result")
                .status,
            HealthStatus::Attention
        );
        assert_eq!(
            critical
                .category_results
                .iter()
                .find(|result| result.category == HealthCategory::Performance)
                .expect("performance result")
                .status,
            HealthStatus::Critical
        );
    }

    #[test]
    fn finding_ids_do_not_depend_on_scan_time() {
        let first = evaluate_health(&inventory(9, 9));
        let mut second_inventory = inventory(9, 9);
        second_inventory.collected_at_epoch_ms += 1000;
        let second = evaluate_health(&second_inventory);

        let first_ids = first
            .findings
            .iter()
            .map(|finding| finding.id.as_str())
            .collect::<Vec<_>>();
        let second_ids = second
            .findings
            .iter()
            .map(|finding| finding.id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(first_ids, second_ids);
    }
}
