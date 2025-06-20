// Storage layer integration tests for ecom functionality
//
// These tests verify the storage layer implementation including:
// - Transaction persistence and retrieval
// - Data integrity and validation
// - Relationship management between entities
// - Query operations and filtering
//
// All tests use unique identifiers to ensure parallel execution safety

pub mod basic_operations_tests;
pub mod integrity_tests;
pub mod relationship_tests;
pub mod query_tests;
pub mod filter_tests;
pub mod transaction_tests;