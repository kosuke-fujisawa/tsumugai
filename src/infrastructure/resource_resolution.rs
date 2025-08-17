//! Infrastructure for resource resolution and asset management

use crate::domain::value_objects::ResourceId;
use std::path::{Path, PathBuf};
use std::collections::HashMap;

/// Trait for resolving logical resource IDs to physical paths
pub trait ResourceResolver: Send + Sync {
    /// Resolve a background music resource
    fn resolve_bgm(&self, resource_id: &ResourceId) -> Option<PathBuf>;

    /// Resolve a sound effect resource
    fn resolve_se(&self, resource_id: &ResourceId) -> Option<PathBuf>;

    /// Resolve an image resource
    fn resolve_image(&self, resource_id: &ResourceId) -> Option<PathBuf>;

    /// Resolve a movie resource
    fn resolve_movie(&self, resource_id: &ResourceId) -> Option<PathBuf>;

    /// Get all available resources of a given type
    fn list_resources(&self, resource_type: ResourceType) -> Vec<ResourceId>;
}

/// Types of resources that can be resolved
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    Bgm,
    Se,
    Image,
    Movie,
}

/// File system-based resource resolver
pub struct FileSystemResourceResolver {
    base_path: PathBuf,
    bgm_extensions: Vec<String>,
    se_extensions: Vec<String>,
    image_extensions: Vec<String>,
    movie_extensions: Vec<String>,
}

impl FileSystemResourceResolver {
    pub fn new<P: Into<PathBuf>>(base_path: P) -> Self {
        Self {
            base_path: base_path.into(),
            bgm_extensions: vec!["mp3".to_string(), "ogg".to_string(), "wav".to_string()],
            se_extensions: vec!["mp3".to_string(), "ogg".to_string(), "wav".to_string()],
            image_extensions: vec!["png".to_string(), "jpg".to_string(), "jpeg".to_string()],
            movie_extensions: vec!["mp4".to_string(), "webm".to_string(), "avi".to_string()],
        }
    }

    pub fn with_extensions<P: Into<PathBuf>>(
        base_path: P,
        bgm_extensions: Vec<String>,
        se_extensions: Vec<String>,
        image_extensions: Vec<String>,
        movie_extensions: Vec<String>,
    ) -> Self {
        Self {
            base_path: base_path.into(),
            bgm_extensions,
            se_extensions,
            image_extensions,
            movie_extensions,
        }
    }

    fn find_file_with_extensions(&self, subdir: &str, name: &str, extensions: &[String]) -> Option<PathBuf> {
        let dir_path = self.base_path.join(subdir);
        
        for ext in extensions {
            let file_path = dir_path.join(format!("{}.{}", name, ext));
            if file_path.exists() {
                return Some(file_path);
            }
        }
        
        None
    }

    fn list_files_in_dir(&self, subdir: &str, extensions: &[String]) -> Vec<ResourceId> {
        let dir_path = self.base_path.join(subdir);
        let mut resources = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&dir_path) {
            for entry in entries.flatten() {
                if let Some(file_name) = entry.file_name().to_str() {
                    if let Some(stem) = Path::new(file_name).file_stem().and_then(|s| s.to_str()) {
                        if let Some(ext) = Path::new(file_name).extension().and_then(|e| e.to_str()) {
                            if extensions.iter().any(|allowed_ext| allowed_ext == ext) {
                                resources.push(ResourceId::from(stem));
                            }
                        }
                    }
                }
            }
        }

        resources
    }
}

impl ResourceResolver for FileSystemResourceResolver {
    fn resolve_bgm(&self, resource_id: &ResourceId) -> Option<PathBuf> {
        self.find_file_with_extensions("bgm", resource_id.as_str(), &self.bgm_extensions)
    }

    fn resolve_se(&self, resource_id: &ResourceId) -> Option<PathBuf> {
        self.find_file_with_extensions("sounds", resource_id.as_str(), &self.se_extensions)
    }

    fn resolve_image(&self, resource_id: &ResourceId) -> Option<PathBuf> {
        self.find_file_with_extensions("images", resource_id.as_str(), &self.image_extensions)
    }

    fn resolve_movie(&self, resource_id: &ResourceId) -> Option<PathBuf> {
        self.find_file_with_extensions("movies", resource_id.as_str(), &self.movie_extensions)
    }

    fn list_resources(&self, resource_type: ResourceType) -> Vec<ResourceId> {
        match resource_type {
            ResourceType::Bgm => self.list_files_in_dir("bgm", &self.bgm_extensions),
            ResourceType::Se => self.list_files_in_dir("sounds", &self.se_extensions),
            ResourceType::Image => self.list_files_in_dir("images", &self.image_extensions),
            ResourceType::Movie => self.list_files_in_dir("movies", &self.movie_extensions),
        }
    }
}

/// In-memory resource resolver for testing
pub struct InMemoryResourceResolver {
    bgm_resources: HashMap<ResourceId, PathBuf>,
    se_resources: HashMap<ResourceId, PathBuf>,
    image_resources: HashMap<ResourceId, PathBuf>,
    movie_resources: HashMap<ResourceId, PathBuf>,
}

impl InMemoryResourceResolver {
    pub fn new() -> Self {
        Self {
            bgm_resources: HashMap::new(),
            se_resources: HashMap::new(),
            image_resources: HashMap::new(),
            movie_resources: HashMap::new(),
        }
    }

    pub fn add_bgm<P: Into<PathBuf>>(&mut self, resource_id: ResourceId, path: P) {
        self.bgm_resources.insert(resource_id, path.into());
    }

    pub fn add_se<P: Into<PathBuf>>(&mut self, resource_id: ResourceId, path: P) {
        self.se_resources.insert(resource_id, path.into());
    }

    pub fn add_image<P: Into<PathBuf>>(&mut self, resource_id: ResourceId, path: P) {
        self.image_resources.insert(resource_id, path.into());
    }

    pub fn add_movie<P: Into<PathBuf>>(&mut self, resource_id: ResourceId, path: P) {
        self.movie_resources.insert(resource_id, path.into());
    }
}

impl Default for InMemoryResourceResolver {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceResolver for InMemoryResourceResolver {
    fn resolve_bgm(&self, resource_id: &ResourceId) -> Option<PathBuf> {
        self.bgm_resources.get(resource_id).cloned()
    }

    fn resolve_se(&self, resource_id: &ResourceId) -> Option<PathBuf> {
        self.se_resources.get(resource_id).cloned()
    }

    fn resolve_image(&self, resource_id: &ResourceId) -> Option<PathBuf> {
        self.image_resources.get(resource_id).cloned()
    }

    fn resolve_movie(&self, resource_id: &ResourceId) -> Option<PathBuf> {
        self.movie_resources.get(resource_id).cloned()
    }

    fn list_resources(&self, resource_type: ResourceType) -> Vec<ResourceId> {
        match resource_type {
            ResourceType::Bgm => self.bgm_resources.keys().cloned().collect(),
            ResourceType::Se => self.se_resources.keys().cloned().collect(),
            ResourceType::Image => self.image_resources.keys().cloned().collect(),
            ResourceType::Movie => self.movie_resources.keys().cloned().collect(),
        }
    }
}

/// Adapter to convert from new ResourceResolver to legacy Resolver trait
pub struct LegacyResolverAdapter {
    resolver: Box<dyn ResourceResolver>,
}

impl LegacyResolverAdapter {
    pub fn new(resolver: Box<dyn ResourceResolver>) -> Self {
        Self { resolver }
    }
}

impl crate::resolve::Resolver for LegacyResolverAdapter {
    fn resolve_bgm(&self, logical: &str) -> Option<PathBuf> {
        let resource_id = ResourceId::from(logical);
        self.resolver.resolve_bgm(&resource_id)
    }

    fn resolve_se(&self, logical: &str) -> Option<PathBuf> {
        let resource_id = ResourceId::from(logical);
        self.resolver.resolve_se(&resource_id)
    }

    fn resolve_image(&self, logical: &str) -> Option<PathBuf> {
        let resource_id = ResourceId::from(logical);
        self.resolver.resolve_image(&resource_id)
    }

    fn resolve_movie(&self, logical: &str) -> Option<PathBuf> {
        let resource_id = ResourceId::from(logical);
        self.resolver.resolve_movie(&resource_id)
    }
}