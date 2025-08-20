//! Dependency injection container for application layer
//!
//! # Dependency Injection Pattern
//!
//! This module provides a simple dependency injection container to manage
//! application services and their dependencies.
//!
//! ## Purpose
//!
//! - Decouple application use cases from concrete infrastructure implementations
//! - Enable easy testing with mock implementations
//! - Centralize service configuration and lifecycle management
//!
//! ## Usage
//!
//! ```rust,ignore
//! use tsumugai::application::dependency_injection::DependencyContainer;
//!
//! let container = DependencyContainer::new()
//!     .register_scenario_repository(repository)
//!     .register_resource_resolver(resolver);
//!
//! let use_case = container.get_scenario_playback_use_case().unwrap();
//! ```
//!
//! ## Current Implementation
//!
//! - Simple HashMap-based container using TypeId for service lookup
//! - Builder pattern for registration
//! - Optional resolution (returns None if service not registered)
//! - Future: Consider more sophisticated DI frameworks for complex scenarios

use super::services::*;
use super::use_cases::*;
use crate::domain::services::StoryExecutionService;
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::sync::Arc;

/// Simple dependency injection container
pub struct DependencyContainer {
    services: HashMap<TypeId, Box<dyn Any + Send + Sync>>,
}

impl DependencyContainer {
    pub fn new() -> Self {
        Self {
            services: HashMap::new(),
        }
    }

    /// Register scenario repository
    pub fn register_scenario_repository(mut self, repo: Arc<dyn ScenarioRepositoryTrait>) -> Self {
        self.services.insert(
            TypeId::of::<Arc<dyn ScenarioRepositoryTrait>>(),
            Box::new(repo),
        );
        self
    }

    /// Register execution service (for future integration when use cases need external story services)
    pub fn register_execution_service(mut self, service: Arc<StoryExecutionService>) -> Self {
        self.services.insert(
            TypeId::of::<Arc<StoryExecutionService>>(),
            Box::new(service),
        );
        self
    }

    /// Register resource resolver
    pub fn register_resource_resolver(mut self, resolver: Arc<dyn ResourceResolverTrait>) -> Self {
        self.services.insert(
            TypeId::of::<Arc<dyn ResourceResolverTrait>>(),
            Box::new(resolver),
        );
        self
    }

    /// Register file repository
    pub fn register_file_repository(mut self, repo: Arc<dyn FileRepositoryTrait>) -> Self {
        self.services
            .insert(TypeId::of::<Arc<dyn FileRepositoryTrait>>(), Box::new(repo));
        self
    }

    /// Register markdown parser
    pub fn register_markdown_parser(mut self, parser: Arc<dyn MarkdownParserTrait>) -> Self {
        self.services.insert(
            TypeId::of::<Arc<dyn MarkdownParserTrait>>(),
            Box::new(parser),
        );
        self
    }

    /// Register save repository
    pub fn register_save_repository(mut self, repo: Arc<dyn SaveRepositoryTrait>) -> Self {
        self.services
            .insert(TypeId::of::<Arc<dyn SaveRepositoryTrait>>(), Box::new(repo));
        self
    }

    /// Resolve a service by type (delegates to get_service for minimal implementation)
    pub fn resolve<T: 'static + Clone>(&self) -> Option<T> {
        self.get_service::<T>()
    }

    /// Get scenario playback use case
    pub fn get_scenario_playback_use_case(&self) -> Option<ScenarioPlaybackUseCase> {
        let scenario_repo = self.get_service::<Arc<dyn ScenarioRepositoryTrait>>()?;
        let resource_resolver = self.get_service::<Arc<dyn ResourceResolverTrait>>()?;

        Some(ScenarioPlaybackUseCase::new(
            scenario_repo,
            resource_resolver,
        ))
    }

    /// Get scenario loading use case
    pub fn get_scenario_loading_use_case(&self) -> Option<ScenarioLoadingUseCase> {
        let file_repo = self.get_service::<Arc<dyn FileRepositoryTrait>>()?;
        let parser = self.get_service::<Arc<dyn MarkdownParserTrait>>()?;

        Some(ScenarioLoadingUseCase::new(file_repo, parser))
    }

    /// Get save game use case
    pub fn get_save_game_use_case(&self) -> Option<SaveGameUseCase> {
        let save_repo = self.get_service::<Arc<dyn SaveRepositoryTrait>>()?;
        Some(SaveGameUseCase::new(save_repo))
    }

    /// Get load game use case
    pub fn get_load_game_use_case(&self) -> Option<LoadGameUseCase> {
        let save_repo = self.get_service::<Arc<dyn SaveRepositoryTrait>>()?;
        Some(LoadGameUseCase::new(save_repo))
    }

    /// Get application coordinator
    pub fn get_application_coordinator(&self) -> Option<ApplicationCoordinator> {
        let story_service = self.get_service::<Arc<StoryExecutionService>>()?;
        let scenario_repo = self.get_service::<Arc<dyn ScenarioRepositoryTrait>>()?;
        let resource_resolver = self.get_service::<Arc<dyn ResourceResolverTrait>>()?;

        Some(ApplicationCoordinator::new(
            story_service,
            scenario_repo,
            resource_resolver,
        ))
    }

    /// Helper to get a service from the container
    fn get_service<T: 'static + Clone>(&self) -> Option<T> {
        self.services
            .get(&TypeId::of::<T>())
            .and_then(|service| service.downcast_ref::<T>())
            .cloned()
    }
}

impl Default for DependencyContainer {
    fn default() -> Self {
        Self::new()
    }
}
