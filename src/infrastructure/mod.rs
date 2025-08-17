//! Infrastructure layer - External dependencies and adapters
//! 
//! This layer contains implementations that deal with external concerns
//! like file systems, parsing, serialization, and other I/O operations.

pub mod md_parser;
pub mod parsing;
pub mod repositories;
pub mod resource_resolution;

pub use md_parser::*;
pub use parsing::*;
pub use repositories::*;
pub use resource_resolution::*;