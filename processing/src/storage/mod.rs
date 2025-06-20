// Re-export all storage-related modules
mod common;
mod web;
mod processible;
mod importable;
mod implementation;

// Re-export storage traits and implementations
pub use common::*;
pub use web::*;
pub use processible::*;
pub use importable::*;
pub use implementation::*;

// Re-export MatcherConfig
pub type MatcherConfig = (i32, i32); // (confidence, importance) 