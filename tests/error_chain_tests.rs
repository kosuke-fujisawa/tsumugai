//! Error chain preservation tests
//! Validates that error sources are properly preserved through the error chain

use std::error::Error;
use std::sync::Arc;
use tsumugai::application::use_cases::{RepositoryError, ScenarioRepositoryTrait};
use tsumugai::domain::{entities::Scenario, value_objects::ScenarioId};

/// Mock repository that always fails for testing error chain preservation
struct FailingScenarioRepository;

#[async_trait::async_trait]
impl ScenarioRepositoryTrait for FailingScenarioRepository {
    async fn load_scenario(&self, id: &ScenarioId) -> Result<Scenario, RepositoryError> {
        // Create a custom error as the source
        let custom_error = std::io::Error::new(std::io::ErrorKind::NotFound, "Custom not found");
        Err(RepositoryError::ScenarioNotFound {
            id: id.as_str().to_string(),
            source: Box::new(custom_error),
        })
    }

    async fn save_scenario(&self, _: &Scenario) -> Result<(), RepositoryError> {
        unimplemented!()
    }

    async fn list_scenarios(&self) -> Result<Vec<ScenarioId>, RepositoryError> {
        unimplemented!()
    }
}

#[tokio::test]
async fn test_error_chain_preserved() {
    // Test that the error source is preserved through the chain
    let repository = Arc::new(FailingScenarioRepository);
    let scenario_id = ScenarioId::new("test".to_string());
    
    let result = repository.load_scenario(&scenario_id).await;
    
    assert!(result.is_err());
    let error = result.unwrap_err();
    
    // Verify the error chain is preserved
    match &error {
        RepositoryError::ScenarioNotFound { id, source } => {
            assert_eq!(id, "test");
            // Verify that we can downcast to the original error type
            assert!(source.downcast_ref::<std::io::Error>().is_some());
            
            // Verify the error message is preserved
            let io_error = source.downcast_ref::<std::io::Error>().unwrap();
            assert_eq!(io_error.kind(), std::io::ErrorKind::NotFound);
        }
        _ => panic!("Expected ScenarioNotFound error"),
    }
    
    // Test the source() method works
    let source_error = error.source();
    assert!(source_error.is_some());
    assert!(source_error.unwrap().downcast_ref::<std::io::Error>().is_some());
}

#[test]
fn test_not_found_constructor() {
    // Test the convenience constructor
    let error = RepositoryError::not_found("test-scenario");
    
    match error {
        RepositoryError::ScenarioNotFound { id, source } => {
            assert_eq!(id, "test-scenario");
            assert!(source.downcast_ref::<std::io::Error>().is_some());
        }
        _ => panic!("Expected ScenarioNotFound error"),
    }
}

#[test]
fn test_error_display() {
    let error = RepositoryError::not_found("display-test");
    let display_string = error.to_string();
    assert!(display_string.contains("Scenario not found: display-test"));
}