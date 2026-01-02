//! Lint configuration

use serde::{Deserialize, Serialize};

/// Lint configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct LintConfig {
    /// Syntax checks configuration
    pub syntax: SyntaxConfig,
    /// Reference checks configuration
    pub references: ReferencesConfig,
    /// Quality checks configuration
    pub quality: QualityConfig,
    /// Flow analysis configuration
    pub flow: FlowConfig,
}


/// Syntax checking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyntaxConfig {
    /// Enable syntax checks
    pub enabled: bool,
    /// Check for required parameters
    pub check_required_params: bool,
}

impl Default for SyntaxConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_required_params: true,
        }
    }
}

/// Reference checking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReferencesConfig {
    /// Enable reference checks
    pub enabled: bool,
    /// Check for undefined labels
    pub check_labels: bool,
    /// Check for undefined variables
    pub check_variables: bool,
}

impl Default for ReferencesConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_labels: true,
            check_variables: false, // Variables are dynamic
        }
    }
}

/// Quality checking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QualityConfig {
    /// Enable quality checks
    pub enabled: bool,
    /// Maximum consecutive WAIT duration (seconds)
    pub max_consecutive_wait: f32,
    /// Warn on duplicate BGM
    pub warn_duplicate_bgm: bool,
    /// Maximum text length (characters)
    pub max_text_length: usize,
}

impl Default for QualityConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_consecutive_wait: 5.0,
            warn_duplicate_bgm: true,
            max_text_length: 200,
        }
    }
}

/// Flow analysis configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowConfig {
    /// Enable flow analysis
    pub enabled: bool,
    /// Check for termination (all paths lead to end)
    pub check_termination: bool,
    /// Check for infinite loops
    pub check_infinite_loops: bool,
    /// Maximum depth for analysis
    pub max_analysis_depth: usize,
}

impl Default for FlowConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            check_termination: false,    // Complex analysis
            check_infinite_loops: false, // Complex analysis
            max_analysis_depth: 100,
        }
    }
}
