//! Runtime state representation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Runtime state of the story execution
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct State {
    /// Program counter - current position in the IR
    pub pc: usize,
    /// Game variables/flags
    pub flags: HashMap<String, serde_json::Value>,
}

impl State {
    /// Create new initial state
    pub fn new() -> Self {
        Self::default()
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
}
