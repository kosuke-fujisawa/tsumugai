//! Resource resolution - mapping logical names to file paths.

use std::path::PathBuf;

pub trait Resolver: Send {
    fn resolve_bgm(&self, logical: &str) -> Option<PathBuf> {
        let _ = logical;
        None
    }

    fn resolve_se(&self, logical: &str) -> Option<PathBuf> {
        let _ = logical;
        None
    }

    fn resolve_image(&self, logical: &str) -> Option<PathBuf> {
        let _ = logical;
        None
    }

    fn resolve_movie(&self, logical: &str) -> Option<PathBuf> {
        let _ = logical;
        None
    }
}

pub struct BasicResolver {
    pub base_dir: PathBuf,
}

impl BasicResolver {
    pub fn new<P: Into<PathBuf>>(base_dir: P) -> Self {
        Self {
            base_dir: base_dir.into(),
        }
    }

    fn find_file(&self, subdir: &str, logical: &str, extensions: &[&str]) -> Option<PathBuf> {
        let dir = self.base_dir.join(subdir);

        for ext in extensions {
            let path = dir.join(format!("{logical}{ext}"));
            if path.exists() {
                return Some(path);
            }
        }

        None
    }
}

impl Resolver for BasicResolver {
    fn resolve_bgm(&self, logical: &str) -> Option<PathBuf> {
        self.find_file("bgm", logical, &[".ogg", ".mp3", ".wav"])
    }

    fn resolve_se(&self, logical: &str) -> Option<PathBuf> {
        self.find_file("se", logical, &[".ogg", ".mp3", ".wav"])
    }

    fn resolve_image(&self, logical: &str) -> Option<PathBuf> {
        self.find_file("images", logical, &[".png", ".jpg", ".webp"])
    }

    fn resolve_movie(&self, logical: &str) -> Option<PathBuf> {
        self.find_file("movies", logical, &[".mp4", ".webm"])
    }
}
