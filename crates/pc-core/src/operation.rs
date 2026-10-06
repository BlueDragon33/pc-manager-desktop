use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupExecutionError {
    pub provider_id: String,
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupProviderExecutionResult {
    pub provider_id: String,
    pub requested_files: u64,
    pub requested_bytes: u64,
    pub deleted_files: u64,
    pub deleted_bytes: u64,
    pub failed_files: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CleanupOperationRecord {
    pub operation_id: String,
    pub plan_id: String,
    pub started_at_epoch_ms: u64,
    pub completed_at_epoch_ms: u64,
    pub requested_files: u64,
    pub requested_bytes: u64,
    pub deleted_files: u64,
    pub deleted_bytes: u64,
    pub failed_files: u64,
    pub provider_results: Vec<CleanupProviderExecutionResult>,
    pub rollback_available: bool,
    pub rollback_label: String,
    pub errors: Vec<CleanupExecutionError>,
}

#[cfg(test)]
mod tests {
    use super::CleanupOperationRecord;

    #[test]
    fn cache_cleanup_record_is_explicitly_not_restorable() {
        let record = CleanupOperationRecord {
            operation_id: "op-1".to_string(),
            plan_id: "plan-1".to_string(),
            started_at_epoch_ms: 1,
            completed_at_epoch_ms: 2,
            requested_files: 1,
            requested_bytes: 10,
            deleted_files: 1,
            deleted_bytes: 10,
            failed_files: 0,
            provider_results: vec![],
            rollback_available: false,
            rollback_label: "Not restorable".to_string(),
            errors: vec![],
        };

        assert!(!record.rollback_available);
        assert_eq!(record.rollback_label, "Not restorable");
    }
}
