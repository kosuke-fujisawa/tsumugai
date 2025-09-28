//! Storage module for saving and loading game state
//!
//! This module provides save/load functionality using JSON serialization.

use crate::types::state::State;

/// Save state to bytes using JSON serialization
pub fn save(state: &State) -> anyhow::Result<Vec<u8>> {
    let json = serde_json::to_string_pretty(state)?;
    Ok(json.into_bytes())
}

/// Load state from bytes using JSON deserialization
pub fn load(bytes: &[u8]) -> anyhow::Result<State> {
    let json = String::from_utf8(bytes.to_vec())?;
    let state = serde_json::from_str(&json)?;
    Ok(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::state::State;

    #[test]
    fn save_then_load_restores_state() {
        let mut original_state = State::new();
        original_state.pc = 5;
        original_state.set_var("score".to_string(), "100".to_string());
        original_state.set_var("name".to_string(), "Alice".to_string());

        // Save the state
        let bytes = save(&original_state).unwrap();

        // Load it back
        let restored_state = load(&bytes).unwrap();

        // Verify it's identical
        assert_eq!(original_state, restored_state);
        assert_eq!(restored_state.pc, 5);
        assert_eq!(restored_state.get_var("score"), Some("100".to_string()));
        assert_eq!(restored_state.get_var("name"), Some("Alice".to_string()));
    }

    #[test]
    fn save_empty_state() {
        let state = State::new();
        let bytes = save(&state).unwrap();
        let restored = load(&bytes).unwrap();
        assert_eq!(state, restored);
    }

    #[test]
    fn load_invalid_data_returns_error() {
        let invalid_bytes = b"invalid json data";
        let result = load(invalid_bytes);
        assert!(result.is_err());
    }
}
