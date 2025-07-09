// Re-export all storage-related modules
pub mod common;
pub mod implementation;
pub mod importable;
pub mod processible;
pub mod web;

// Re-export storage traits and implementations
pub use common::*;
pub use web::*;
pub use processible::*;
pub use importable::*;
pub use implementation::*;

// Re-export MatcherConfig
pub type MatcherConfig = (i32, i32); // (confidence, importance) 