use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanRootSelection {
    pub id: String,
    pub display_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateScanOptions {
    #[serde(default = "default_min_file_bytes")]
    pub min_file_bytes: u64,
    #[serde(default)]
    pub excluded_directory_names: Vec<String>,
    #[serde(default = "default_max_groups")]
    pub max_groups: usize,
    #[serde(default = "default_max_files_per_group")]
    pub max_files_per_group: usize,
}

impl Default for DuplicateScanOptions {
    fn default() -> Self {
        Self {
            min_file_bytes: default_min_file_bytes(),
            excluded_directory_names: Vec::new(),
            max_groups: default_max_groups(),
            max_files_per_group: default_max_files_per_group(),
        }
    }
}

const fn default_min_file_bytes() -> u64 {
    1_024
}

const fn default_max_groups() -> usize {
    200
}

const fn default_max_files_per_group() -> usize {
    50
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilesystemWarning {
    pub source: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateFile {
    pub id: String,
    pub path: String,
    pub bytes: u64,
    pub modified_at_epoch_ms: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    pub id: String,
    pub bytes_each: u64,
    pub total_bytes: u64,
    pub recoverable_bytes: u64,
    pub files: Vec<DuplicateFile>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateScanSummary {
    pub scan_id: String,
    pub scanned_files: u64,
    pub scanned_bytes: u64,
    pub duplicate_groups: Vec<DuplicateGroup>,
    pub warnings: Vec<FilesystemWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicatePlanItem {
    pub id: String,
    pub group_id: String,
    pub root: String,
    pub path: String,
    pub bytes: u64,
    pub modified_at_epoch_ms: Option<u64>,
    pub full_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateScanPlan {
    pub scan_id: String,
    pub items: Vec<DuplicatePlanItem>,
}

#[derive(Debug, Default)]
pub struct DuplicateScanPlanStore {
    latest: Option<DuplicateScanPlan>,
}

impl DuplicateScanPlanStore {
    pub fn replace(&mut self, plan: DuplicateScanPlan) {
        self.latest = Some(plan);
    }

    #[must_use]
    pub fn get_by_id(&self, scan_id: &str) -> Option<&DuplicateScanPlan> {
        self.latest.as_ref().filter(|plan| plan.scan_id == scan_id)
    }

    pub fn take_by_id(&mut self, scan_id: &str) -> Option<DuplicateScanPlan> {
        if self
            .latest
            .as_ref()
            .is_some_and(|plan| plan.scan_id == scan_id)
        {
            self.latest.take()
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateDeleteRequest {
    pub scan_id: String,
    pub file_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateDeleteError {
    pub file_id: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateDeleteResult {
    pub operation_id: String,
    pub scan_id: String,
    pub requested_files: u64,
    pub deleted_files: u64,
    pub deleted_bytes: u64,
    pub failed_files: u64,
    pub errors: Vec<DuplicateDeleteError>,
    pub completed_at_epoch_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageScanOptions {
    #[serde(default)]
    pub excluded_directory_names: Vec<String>,
    #[serde(default = "default_top_files")]
    pub top_files: usize,
    #[serde(default = "default_top_folders")]
    pub top_folders: usize,
}

impl Default for StorageScanOptions {
    fn default() -> Self {
        Self {
            excluded_directory_names: Vec::new(),
            top_files: default_top_files(),
            top_folders: default_top_folders(),
        }
    }
}

const fn default_top_files() -> usize {
    50
}

const fn default_top_folders() -> usize {
    50
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageFileEntry {
    pub path: String,
    pub bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageFolderAggregate {
    pub path: String,
    pub bytes: u64,
    pub file_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageTypeAggregate {
    pub extension: String,
    pub bytes: u64,
    pub file_count: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageScanSummary {
    pub scan_id: String,
    pub total_files: u64,
    pub total_bytes: u64,
    pub largest_files: Vec<StorageFileEntry>,
    pub largest_folders: Vec<StorageFolderAggregate>,
    pub file_types: Vec<StorageTypeAggregate>,
    pub warnings: Vec<FilesystemWarning>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FilesystemError {
    pub code: String,
    pub message: String,
    pub recoverable: bool,
}

impl FilesystemError {
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>, recoverable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            recoverable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{DuplicateScanPlan, DuplicateScanPlanStore};

    #[test]
    fn duplicate_plan_store_requires_matching_scan_id() {
        let mut store = DuplicateScanPlanStore::default();
        store.replace(DuplicateScanPlan {
            scan_id: "scan-a".to_string(),
            items: Vec::new(),
        });

        assert!(store.get_by_id("scan-a").is_some());
        assert!(store.get_by_id("scan-b").is_none());
        assert!(store.take_by_id("scan-b").is_none());
        assert!(store.take_by_id("scan-a").is_some());
    }
}
