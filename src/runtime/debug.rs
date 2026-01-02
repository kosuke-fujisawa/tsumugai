//! Debug logging for runtime execution
//!
//! Provides detailed logging capabilities for debugging scenario execution.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Debug log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    /// All internal state changes
    Trace,
    /// Development debugging information
    Debug,
    /// Important state changes
    Info,
    /// Potential issues
    Warn,
    /// Error situations
    Error,
}

/// Debug log category
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum DebugCategory {
    /// Engine execution
    Engine,
    /// Variable operations
    Variables,
    /// Resource resolution
    Resources,
    /// Performance metrics
    Performance,
    /// Control flow (jumps, branches)
    Flow,
}

/// Debug configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugConfig {
    /// Enable debug logging
    pub enabled: bool,
    /// Minimum log level
    pub level: LogLevel,
    /// Output destination
    pub output: DebugOutput,
    /// Enabled categories
    pub categories: HashSet<DebugCategory>,
}

impl Default for DebugConfig {
    fn default() -> Self {
        let mut categories = HashSet::new();
        categories.insert(DebugCategory::Engine);
        categories.insert(DebugCategory::Flow);

        Self {
            enabled: std::env::var("TSUMUGAI_DEBUG").is_ok(),
            level: LogLevel::Debug,
            output: DebugOutput::Stderr,
            categories,
        }
    }
}

/// Debug output destination
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DebugOutput {
    /// Output to stderr
    Stderr,
    /// Output to file
    File(String),
}

/// Debug snapshot of current engine state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebugSnapshot {
    /// Program counter
    pub pc: usize,
    /// Current step type
    pub step_type: String,
    /// Variables state
    pub variables: std::collections::HashMap<String, String>,
    /// Waiting for choice
    pub waiting_for_choice: bool,
    /// Pending choices
    pub pending_choices: Vec<String>,
}

/// Log a debug message
pub fn log(config: &DebugConfig, category: DebugCategory, level: LogLevel, message: &str) {
    if !config.enabled {
        return;
    }

    if !config.categories.contains(&category) {
        return;
    }

    match &config.output {
        DebugOutput::Stderr => {
            let level_str = match level {
                LogLevel::Trace => "TRACE",
                LogLevel::Debug => "DEBUG",
                LogLevel::Info => "INFO",
                LogLevel::Warn => "WARN",
                LogLevel::Error => "ERROR",
            };
            let category_str = format!("{:?}", category);
            eprintln!("[{}] {:10} {}", level_str, category_str, message);
        }
        DebugOutput::File(_path) => {
            // File output not yet implemented
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_config_default() {
        let config = DebugConfig::default();
        assert!(!config.enabled || std::env::var("TSUMUGAI_DEBUG").is_ok());
        assert!(config.categories.contains(&DebugCategory::Engine));
    }

    #[test]
    fn debug_config_enable_all_categories() {
        let mut config = DebugConfig::default();
        config.enabled = true;
        config.categories.insert(DebugCategory::Variables);
        config.categories.insert(DebugCategory::Performance);

        assert!(config.categories.contains(&DebugCategory::Variables));
        assert!(config.categories.contains(&DebugCategory::Performance));
    }

    #[test]
    fn debug_log_output() {
        let config = DebugConfig {
            enabled: true,
            level: LogLevel::Debug,
            output: DebugOutput::Stderr,
            categories: {
                let mut set = HashSet::new();
                set.insert(DebugCategory::Engine);
                set
            },
        };

        // This should output to stderr (visible in test output with --nocapture)
        log(
            &config,
            DebugCategory::Engine,
            LogLevel::Debug,
            "Test message",
        );
    }
}
