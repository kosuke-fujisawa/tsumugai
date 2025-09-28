//! Runtime state representation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Runtime state of the story execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct State {
    /// Program counter - current position in the AST
    pub pc: usize,
    /// Game variables/flags
    pub flags: HashMap<String, serde_json::Value>,
    /// Random number generator seed for deterministic execution
    pub rng_seed: u64,
    /// Whether we're currently waiting for a user choice
    pub waiting_for_choice: bool,
    /// Pending choice options if waiting for branch
    pub pending_choices: Vec<String>,
    /// Last label that was reached (for tracking)
    pub last_label: Option<String>,
}

impl State {
    /// Create new initial state
    pub fn new() -> Self {
        Self::default()
    }

    /// Create state with specific seed
    pub fn with_seed(seed: u64) -> Self {
        Self {
            rng_seed: seed,
            ..Default::default()
        }
    }

    /// Get variable value as string
    pub fn get_var(&self, name: &str) -> Option<String> {
        self.flags.get(name).and_then(|v| match v {
            serde_json::Value::String(s) => Some(s.clone()),
            serde_json::Value::Number(n) => Some(n.to_string()),
            serde_json::Value::Bool(b) => Some(b.to_string()),
            _ => None,
        })
    }

    /// Set variable value
    pub fn set_var(&mut self, name: String, value: String) {
        // Try to parse as number first, then store as string
        if let Ok(n) = value.parse::<i64>() {
            self.flags.insert(name, serde_json::Value::Number(n.into()));
        } else if let Ok(f) = value.parse::<f64>() {
            self.flags.insert(name, serde_json::json!(f));
        } else if let Ok(b) = value.parse::<bool>() {
            self.flags.insert(name, serde_json::Value::Bool(b));
        } else {
            self.flags.insert(name, serde_json::Value::String(value));
        }
    }

    /// Modify variable with operation
    pub fn modify_var(
        &mut self,
        name: &str,
        op: crate::types::ast::Operation,
        value: &str,
    ) -> Result<(), String> {
        let current = self.get_var(name).unwrap_or_else(|| "0".to_string());

        // Parse both current and new value as numbers
        let current_num: f64 = current
            .parse()
            .map_err(|_| format!("Cannot parse '{}' as number", current))?;
        let value_num: f64 = value
            .parse()
            .map_err(|_| format!("Cannot parse '{}' as number", value))?;

        let result = match op {
            crate::types::ast::Operation::Add => current_num + value_num,
            crate::types::ast::Operation::Subtract => current_num - value_num,
            crate::types::ast::Operation::Multiply => current_num * value_num,
            crate::types::ast::Operation::Divide => {
                if value_num == 0.0 {
                    return Err("Division by zero".to_string());
                }
                current_num / value_num
            }
        };

        // Store result as integer if it's a whole number, otherwise as float
        if result.fract() == 0.0 {
            self.set_var(name.to_string(), (result as i64).to_string());
        } else {
            self.set_var(name.to_string(), result.to_string());
        }

        Ok(())
    }

    /// Check if condition is true
    pub fn check_condition(
        &self,
        var: &str,
        cmp: &crate::types::ast::Comparison,
        value: &str,
    ) -> Result<bool, String> {
        let current = self.get_var(var).unwrap_or_else(|| "0".to_string());

        // Try numeric comparison first
        if let (Ok(current_num), Ok(value_num)) = (current.parse::<f64>(), value.parse::<f64>()) {
            use crate::types::ast::Comparison;
            return Ok(match cmp {
                Comparison::Equal => (current_num - value_num).abs() < f64::EPSILON,
                Comparison::NotEqual => (current_num - value_num).abs() >= f64::EPSILON,
                Comparison::LessThan => current_num < value_num,
                Comparison::LessThanOrEqual => current_num <= value_num,
                Comparison::GreaterThan => current_num > value_num,
                Comparison::GreaterThanOrEqual => current_num >= value_num,
            });
        }

        // Fall back to string comparison
        use crate::types::ast::Comparison;
        Ok(match cmp {
            Comparison::Equal => current == value,
            Comparison::NotEqual => current != value,
            _ => return Err("Cannot perform numeric comparison on non-numeric values".to_string()),
        })
    }
}
