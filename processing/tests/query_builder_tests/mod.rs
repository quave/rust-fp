use std::sync::OnceLock;
use std::collections::HashMap;

use processing::model::ModelRegistryProvider;
use processing::ui_model::*;
use processing::ui_model::query_builder::SqlQueryBuilder;

// Test modules
pub mod basic_operators;
pub mod logical_operators;
pub mod relationship_queries;
pub mod advanced_queries;

// Registry Provider - shared across all query builder tests
static TEST_REGISTRY: OnceLock<ModelRegistry> = OnceLock::new();

pub struct TestRegistryProvider;

impl ModelRegistryProvider for TestRegistryProvider {
    fn get_registry() -> &'static ModelRegistry {
        TEST_REGISTRY.get_or_init(|| {
            // Create the TestOrder table with the expected alias 't'
            let order_table = Table {
                name: "orders",
                alias: "t".to_string(),
                fields: vec![
                    Field { name: "id", field_type: FieldType::Number },
                    Field { name: "customer_id", field_type: FieldType::Number },
                    Field { name: "amount", field_type: FieldType::Number },
                    Field { name: "created_at", field_type: FieldType::DateTime },
                    Field { name: "status", field_type: FieldType::String },
                ],
                relations: {
                    let mut relations = HashMap::new();
                    relations.insert(
                        "customer".to_string(),
                        Relation {
                            kind: RelationKind::BelongsTo,
                            target: "TestCustomer",
                            foreign_key: "customer_id",
                        },
                    );
                    relations.insert(
                        "items".to_string(),
                        Relation {
                            kind: RelationKind::HasMany,
                            target: "TestOrderItem",
                            foreign_key: "order_id",
                        },
                    );
                    relations
                },
                primary_key: "id",
            };
            
            // Create TestCustomer table with correct fields
            let customer_table = Table {
                name: "customers",
                alias: "customers".to_string(),
                fields: vec![
                    Field { name: "id", field_type: FieldType::Number },
                    Field { name: "name", field_type: FieldType::String },
                    Field { name: "email", field_type: FieldType::String },
                    Field { name: "is_active", field_type: FieldType::Boolean },
                ],
                relations: {
                    let mut relations = HashMap::new();
                    relations.insert(
                        "orders".to_string(),
                        Relation {
                            kind: RelationKind::HasMany,
                            target: "TestOrder",
                            foreign_key: "customer_id",
                        },
                    );
                    relations
                },
                primary_key: "id",
            };

            // Create TestOrderItem table with correct fields
            let order_item_table = Table {
                name: "order_items",
                alias: "order_items".to_string(),
                fields: vec![
                    Field { name: "id", field_type: FieldType::Number },
                    Field { name: "order_id", field_type: FieldType::Number },
                    Field { name: "product_name", field_type: FieldType::String },
                    Field { name: "quantity", field_type: FieldType::Number },
                    Field { name: "price", field_type: FieldType::Number },
                ],
                relations: {
                    let mut relations = HashMap::new();
                    relations.insert(
                        "order".to_string(),
                        Relation {
                            kind: RelationKind::BelongsTo,
                            target: "TestOrder",
                            foreign_key: "order_id",
                        },
                    );
                    relations
                },
                primary_key: "id",
            };

            // Create registry with the root table
            let mut registry = ModelRegistry::new(order_table.clone());
            
            // Register models using the exact target names from relations
            registry.models.insert("TestOrder", order_table);
            registry.models.insert("TestCustomer", customer_table);
            registry.models.insert("TestOrderItem", order_item_table);
            
            registry
        })
    }
}

// Helper function to get only the query string for easier testing
pub fn get_query_string(request: FilterRequest) -> String {
    let (query, _) = SqlQueryBuilder::<TestRegistryProvider>::build_query(&request).unwrap();
    query
} 