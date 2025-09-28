//! External events that can be sent to the runtime

use serde::{Deserialize, Serialize};

/// External events that can be sent to the runtime to trigger state changes
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Event {
    /// User chose an option from a branch
    Choice { id: String },
    /// Continue execution (e.g., user pressed Enter)
    Continue,
    /// Save game to specific slot
    Save { slot: u8 },
    /// Load game from specific slot
    Load { slot: u8 },
}