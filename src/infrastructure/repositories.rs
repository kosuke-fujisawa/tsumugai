//! Infrastructure implementations of repository traits

use crate::domain::entities::{ExecutionSnapshot, Scenario};
use crate::domain::repositories::{RepositoryError, SaveDataRepository, ScenarioRepository};
use crate::domain::value_objects::ScenarioId;
use crate::infrastructure::parsing::{MarkdownScenarioParser, ScenarioParser};
use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;

/// File system implementation of ScenarioRepository
pub struct FileSystemScenarioRepository {
    base_path: PathBuf,
    parser: Box<dyn ScenarioParser + Send + Sync>,
}

impl FileSystemScenarioRepository {
    pub fn new<P: Into<PathBuf>>(base_path: P) -> Self {
        Self {
            base_path: base_path.into(),
            parser: Box::new(MarkdownScenarioParser::with_default_id_generator()),
        }
    }

    pub fn with_parser<P: Into<PathBuf>>(
        base_path: P,
        parser: Box<dyn ScenarioParser + Send + Sync>,
    ) -> Self {
        Self {
            base_path: base_path.into(),
            parser,
        }
    }

    fn get_scenario_path(&self, id: &ScenarioId) -> PathBuf {
        // Simple implementation - look for files with the ID as name
        for ext in self.parser.supported_extensions() {
            let path = self.base_path.join(format!("{}.{}", id.as_str(), ext));
            if path.exists() {
                return path;
            }
        }
        // Default to markdown
        self.base_path.join(format!("{}.md", id.as_str()))
    }
}

#[async_trait]
impl ScenarioRepository for FileSystemScenarioRepository {
    async fn load_scenario(&self, id: &ScenarioId) -> Result<Scenario, RepositoryError> {
        let path = self.get_scenario_path(id);

        if !path.exists() {
            return Err(RepositoryError::not_found(id.clone()));
        }

        let content =
            tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| RepositoryError::IoError {
                    message: format!("Failed to read scenario file {}: {}", path.display(), e),
                })?;

        self.parser
            .parse(&content)
            .await
            .map_err(|e| RepositoryError::InvalidFormat {
                message: format!("Failed to parse scenario: {e}"),
            })
    }

    async fn save_scenario(&self, _scenario: &Scenario) -> Result<(), RepositoryError> {
        // For now, we don't implement saving back to markdown
        // This would require a markdown generator/serializer
        Err(RepositoryError::Unavailable {
            reason: "Saving scenarios to markdown not yet implemented".to_string(),
        })
    }

    async fn scenario_exists(&self, id: &ScenarioId) -> Result<bool, RepositoryError> {
        let path = self.get_scenario_path(id);
        Ok(path.exists())
    }

    async fn list_scenarios(&self) -> Result<Vec<ScenarioId>, RepositoryError> {
        let mut scenarios = Vec::new();
        let extensions = self.parser.supported_extensions();

        let mut entries =
            tokio::fs::read_dir(&self.base_path)
                .await
                .map_err(|e| RepositoryError::IoError {
                    message: format!(
                        "Failed to read directory {}: {}",
                        self.base_path.display(),
                        e
                    ),
                })?;

        while let Some(entry) =
            entries
                .next_entry()
                .await
                .map_err(|e| RepositoryError::IoError {
                    message: format!("Failed to read directory entry: {e}"),
                })?
        {
            let path = entry.path();
            if let Some(extension) = path.extension().and_then(|ext| ext.to_str())
                && extensions.contains(&extension)
                && let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    scenarios.push(ScenarioId::from(stem));
                }
        }

        Ok(scenarios)
    }

    async fn delete_scenario(&self, id: &ScenarioId) -> Result<(), RepositoryError> {
        let path = self.get_scenario_path(id);

        if !path.exists() {
            return Err(RepositoryError::not_found(id.clone()));
        }

        tokio::fs::remove_file(&path)
            .await
            .map_err(|e| RepositoryError::IoError {
                message: format!("Failed to delete scenario file {}: {}", path.display(), e),
            })
    }
}

/// In-memory implementation for testing
pub struct InMemoryScenarioRepository {
    scenarios: HashMap<ScenarioId, Scenario>,
}

impl InMemoryScenarioRepository {
    pub fn new() -> Self {
        Self {
            scenarios: HashMap::new(),
        }
    }

    pub fn add_scenario(&mut self, scenario: Scenario) {
        let id = scenario.id().clone();
        self.scenarios.insert(id, scenario);
    }
}

impl Default for InMemoryScenarioRepository {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl ScenarioRepository for InMemoryScenarioRepository {
    async fn load_scenario(&self, id: &ScenarioId) -> Result<Scenario, RepositoryError> {
        self.scenarios
            .get(id)
            .cloned()
            .ok_or_else(|| RepositoryError::not_found(id.clone()))
    }

    async fn save_scenario(&self, _scenario: &Scenario) -> Result<(), RepositoryError> {
        // In-memory repository is read-only for simplicity
        Err(RepositoryError::Unavailable {
            reason: "In-memory repository is read-only".to_string(),
        })
    }

    async fn scenario_exists(&self, id: &ScenarioId) -> Result<bool, RepositoryError> {
        Ok(self.scenarios.contains_key(id))
    }

    async fn list_scenarios(&self) -> Result<Vec<ScenarioId>, RepositoryError> {
        Ok(self.scenarios.keys().cloned().collect())
    }

    async fn delete_scenario(&self, _id: &ScenarioId) -> Result<(), RepositoryError> {
        Err(RepositoryError::Unavailable {
            reason: "In-memory repository is read-only".to_string(),
        })
    }
}

/// Simple file-based save data repository
pub struct JsonSaveDataRepository {
    base_path: PathBuf,
}

impl JsonSaveDataRepository {
    pub fn new<P: Into<PathBuf>>(base_path: P) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    fn get_save_path(&self, scenario_id: &ScenarioId) -> PathBuf {
        self.base_path
            .join(format!("{}.save.json", scenario_id.as_str()))
    }
}

#[async_trait]
impl SaveDataRepository for JsonSaveDataRepository {
    async fn save_snapshot(
        &self,
        scenario_id: &ScenarioId,
        snapshot: &ExecutionSnapshot,
    ) -> Result<(), RepositoryError> {
        let path = self.get_save_path(scenario_id);

        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| RepositoryError::IoError {
                    message: format!("Failed to create save directory: {e}"),
                })?;
        }

        let json = serde_json::to_string_pretty(snapshot).map_err(|e| {
            RepositoryError::SerializationError {
                message: format!("Failed to serialize snapshot: {e}"),
            }
        })?;

        tokio::fs::write(&path, json)
            .await
            .map_err(|e| RepositoryError::IoError {
                message: format!("Failed to write save file {}: {}", path.display(), e),
            })
    }

    async fn load_snapshot(
        &self,
        scenario_id: &ScenarioId,
    ) -> Result<Option<ExecutionSnapshot>, RepositoryError> {
        let path = self.get_save_path(scenario_id);

        if !path.exists() {
            return Ok(None);
        }

        let content =
            tokio::fs::read_to_string(&path)
                .await
                .map_err(|e| RepositoryError::IoError {
                    message: format!("Failed to read save file {}: {}", path.display(), e),
                })?;

        let snapshot =
            serde_json::from_str(&content).map_err(|e| RepositoryError::SerializationError {
                message: format!("Failed to deserialize snapshot: {e}"),
            })?;

        Ok(Some(snapshot))
    }

    async fn list_snapshots(
        &self,
        scenario_id: &ScenarioId,
    ) -> Result<Vec<ExecutionSnapshot>, RepositoryError> {
        // For this simple implementation, we only support one snapshot per scenario
        match self.load_snapshot(scenario_id).await? {
            Some(snapshot) => Ok(vec![snapshot]),
            None => Ok(vec![]),
        }
    }

    async fn delete_snapshot(&self, scenario_id: &ScenarioId) -> Result<(), RepositoryError> {
        let path = self.get_save_path(scenario_id);

        if !path.exists() {
            return Err(RepositoryError::SaveDataNotFound {
                id: scenario_id.clone(),
            });
        }

        tokio::fs::remove_file(&path)
            .await
            .map_err(|e| RepositoryError::IoError {
                message: format!("Failed to delete save file {}: {}", path.display(), e),
            })
    }
}
