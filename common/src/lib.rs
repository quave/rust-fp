pub mod config;
pub mod yaml_include;

/// Common utilities shared across the Frida AI project
///
/// This crate provides shared functionality that can be used across different
/// modules of the Frida AI fraud detection system, including:
///
/// - Database connection management
/// - Shared test utilities and mocks
/// - Common data structures and utilities
/// - Test data factories and helpers

// Test helpers module - available for both development and test builds
#[cfg(any(test, feature = "test-helpers"))]
pub mod test_helpers;

// Re-export commonly used test utilities for easier access
#[cfg(any(test, feature = "test-helpers"))]
pub use test_helpers::{
    create_test_connection, generate_unique_id, generate_unique_test_id, get_test_database_url,
    truncate_tables,
};

/// Centralized test utilities and mocks for the Frida AI project
///
/// This module provides shared test utilities, mocks, and database helpers
/// to eliminate code duplication across test files.
pub mod docs {
    //! Documentation and examples for the centralized test utilities
}
