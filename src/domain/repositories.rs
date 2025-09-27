//! Domain repository traits - Abstractions for data persistence

use crate::domain::entities::{ExecutionSnapshot, Scenario};
use crate::domain::errors::DomainError;
use crate::domain::value_objects::ScenarioId;
use async_trait::async_trait;

/// Repository for scenario persistence
///
/// This trait defines the contract for scenario storage and retrieval,
/// without specifying implementation details (file system, database, etc.)
#[async_trait]
pub trait ScenarioRepository: Send + Sync {
    /// Load a scenario by its ID
    async fn load_scenario(&self, id: &ScenarioId) -> Result<Scenario, RepositoryError>;

    /// Save a scenario
    async fn save_scenario(&self, scenario: &Scenario) -> Result<(), RepositoryError>;

    /// Check if a scenario exists
    async fn scenario_exists(&self, id: &ScenarioId) -> Result<bool, RepositoryError>;

    /// List all available scenario IDs
    async fn list_scenarios(&self) -> Result<Vec<ScenarioId>, RepositoryError>;

    /// Delete a scenario
    async fn delete_scenario(&self, id: &ScenarioId) -> Result<(), RepositoryError>;
}

/// Repository for save data persistence
#[async_trait]
pub trait SaveDataRepository: Send + Sync {
    /// Save execution state
    async fn save_snapshot(
        &self,
        scenario_id: &ScenarioId,
        snapshot: &ExecutionSnapshot,
    ) -> Result<(), RepositoryError>;

    /// Load execution state
    async fn load_snapshot(
        &self,
        scenario_id: &ScenarioId,
    ) -> Result<Option<ExecutionSnapshot>, RepositoryError>;

    /// List all saved snapshots for a scenario
    async fn list_snapshots(
        &self,
        scenario_id: &ScenarioId,
    ) -> Result<Vec<ExecutionSnapshot>, RepositoryError>;

    /// Delete saved state
    async fn delete_snapshot(&self, scenario_id: &ScenarioId) -> Result<(), RepositoryError>;
}

/// Repository errors
#[derive(Debug, thiserror::Error)]
pub enum RepositoryError {
    #[error("Scenario not found: {id}")]
    ScenarioNotFound {
        id: ScenarioId,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },

    #[error("Save data not found for scenario: {id}")]
    SaveDataNotFound { id: ScenarioId },

    #[error("IO error: {message}")]
    IoError { message: String },

    #[error("Serialization error: {message}")]
    SerializationError { message: String },

    #[error("Permission denied: {resource}")]
    PermissionDenied { resource: String },

    #[error("Storage full: cannot save {resource}")]
    StorageFull { resource: String },

    #[error("Invalid data format: {message}")]
    InvalidFormat { message: String },

    #[error("Repository unavailable: {reason}")]
    Unavailable { reason: String },
}

impl RepositoryError {
    /// Create a not found error with an optional source
    pub fn not_found(id: impl Into<ScenarioId>) -> Self {
        Self::ScenarioNotFound {
            id: id.into(),
            source: Some(Box::new(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Scenario not found",
            ))),
        }
    }
}

impl From<RepositoryError> for DomainError {
    fn from(error: RepositoryError) -> Self {
        match error {
            RepositoryError::ScenarioNotFound { id, .. } => {
                DomainError::invalid_scenario(format!("Scenario not found: {}", id.as_str()))
            }
            RepositoryError::SaveDataNotFound { id } => {
                DomainError::execution_state_error(format!("Save data not found: {}", id.as_str()))
            }
            _ => DomainError::execution_state_error(error.to_string()),
        }
    }
}
