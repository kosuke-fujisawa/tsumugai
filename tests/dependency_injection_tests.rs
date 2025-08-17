//! Dependency injection tests
//! Tests the resolve functionality and service registration

use std::sync::Arc;
use tsumugai::application::{
    dependency_injection::DependencyContainer,
    use_cases::{RepositoryError, ResourceResolverTrait, ScenarioRepositoryTrait},
};
use tsumugai::domain::{entities::Scenario, value_objects::ScenarioId};

/// Mock repository for testing DI
struct TestScenarioRepository;

#[async_trait::async_trait]
impl ScenarioRepositoryTrait for TestScenarioRepository {
    async fn load_scenario(&self, _: &ScenarioId) -> Result<Scenario, RepositoryError> {
        Ok(Scenario::new(
            ScenarioId::new("test".to_string()),
            "Test".to_string(),
            vec![],
        ))
    }

    async fn save_scenario(&self, _: &Scenario) -> Result<(), RepositoryError> {
        Ok(())
    }

    async fn list_scenarios(&self) -> Result<Vec<ScenarioId>, RepositoryError> {
        Ok(vec![])
    }
}

/// Mock resource resolver for testing DI
struct TestResourceResolver;

impl ResourceResolverTrait for TestResourceResolver {
    fn resolve_bgm(&self, _: &str) -> Option<std::path::PathBuf> {
        None
    }

    fn resolve_se(&self, _: &str) -> Option<std::path::PathBuf> {
        None
    }

    fn resolve_image(&self, _: &str) -> Option<std::path::PathBuf> {
        None
    }

    fn resolve_movie(&self, _: &str) -> Option<std::path::PathBuf> {
        None
    }
}

#[test]
fn test_resolve_min_impl() {
    let mut container = DependencyContainer::new();

    // Register a service
    let repo: Arc<dyn ScenarioRepositoryTrait> = Arc::new(TestScenarioRepository);
    container = container.register_scenario_repository(repo.clone());

    // Resolve should return the registered service
    let resolved: Option<Arc<dyn ScenarioRepositoryTrait>> = container.resolve();
    assert!(resolved.is_some());

    // Check that it's the same instance (by checking Arc pointer equality)
    let resolved = resolved.unwrap();
    assert!(Arc::ptr_eq(&repo, &resolved));
}

#[test]
fn test_resolve_returns_none_for_unregistered() {
    let container = DependencyContainer::new();

    // Try to resolve a service that wasn't registered
    let resolved: Option<Arc<dyn ResourceResolverTrait>> = container.resolve();
    assert!(resolved.is_none());
}

#[test]
fn test_resolve_multiple_services() {
    let mut container = DependencyContainer::new();

    // Register multiple services
    let repo: Arc<dyn ScenarioRepositoryTrait> = Arc::new(TestScenarioRepository);
    let resolver: Arc<dyn ResourceResolverTrait> = Arc::new(TestResourceResolver);

    container = container
        .register_scenario_repository(repo.clone())
        .register_resource_resolver(resolver.clone());

    // Both should be resolvable
    let resolved_repo: Option<Arc<dyn ScenarioRepositoryTrait>> = container.resolve();
    let resolved_resolver: Option<Arc<dyn ResourceResolverTrait>> = container.resolve();

    assert!(resolved_repo.is_some());
    assert!(resolved_resolver.is_some());

    // Check they're the correct instances
    assert!(Arc::ptr_eq(&repo, &resolved_repo.unwrap()));
    assert!(Arc::ptr_eq(&resolver, &resolved_resolver.unwrap()));
}

#[test]
fn test_get_scenario_playback_use_case() {
    let mut container = DependencyContainer::new();

    // Register required dependencies
    let repo: Arc<dyn ScenarioRepositoryTrait> = Arc::new(TestScenarioRepository);
    let resolver: Arc<dyn ResourceResolverTrait> = Arc::new(TestResourceResolver);

    container = container
        .register_scenario_repository(repo)
        .register_resource_resolver(resolver);

    // Should be able to create the use case
    let use_case = container.get_scenario_playback_use_case();
    assert!(use_case.is_some());
}

#[test]
fn test_get_scenario_playback_use_case_fails_without_dependencies() {
    let container = DependencyContainer::new();

    // Should fail without registered dependencies
    let use_case = container.get_scenario_playback_use_case();
    assert!(use_case.is_none());
}
