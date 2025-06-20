use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Operator {
    #[serde(rename = "=")]
    Equal,
    #[serde(rename = "!=")]
    NotEqual,
    #[serde(rename = ">")]
    GreaterThan,
    #[serde(rename = ">=")]
    GreaterThanOrEqual,
    #[serde(rename = "<")]
    LessThan,
    #[serde(rename = "<=")]
    LessThanOrEqual,
    #[serde(rename = "like")]
    Like,
    #[serde(rename = "in")]
    In,
    #[serde(rename = "not_in")]
    NotIn,
    #[serde(rename = "between")]
    Between,
    #[serde(rename = "is_null")]
    IsNull,
    #[serde(rename = "is_not_null")]
    IsNotNull,
}

impl Operator {
    pub fn to_sql(&self) -> &'static str {
        match self {
            Operator::Equal => "=",
            Operator::NotEqual => "!=",
            Operator::GreaterThan => ">",
            Operator::GreaterThanOrEqual => ">=",
            Operator::LessThan => "<",
            Operator::LessThanOrEqual => "<=",
            Operator::Like => "LIKE",
            Operator::In => "IN",
            Operator::NotIn => "NOT IN",
            Operator::Between => "BETWEEN",
            Operator::IsNull => "IS NULL",
            Operator::IsNotNull => "IS NOT NULL",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum FilterValue {
    String(String),
    Number(f64),
    Boolean(bool),
    StringArray(Vec<String>),
    NumberArray(Vec<f64>),
    Range { min: f64, max: f64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterCondition {
    pub column: String,
    pub operator: Operator,
    pub value: FilterValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LogicalOperator {
    #[serde(rename = "and")]
    And,
    #[serde(rename = "or")]
    Or,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterGroup {
    pub operator: LogicalOperator,
    pub conditions: Vec<FilterCondition>,
    #[serde(default)]
    pub groups: Vec<FilterGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortOrder {
    pub column: String,
    pub direction: SortDirection,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SortDirection {
    #[serde(rename = "asc")]
    Ascending,
    #[serde(rename = "desc")]
    Descending,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilterRequest {
    #[serde(default)]
    pub filter: Option<FilterGroup>,
    #[serde(default)]
    pub sort: Vec<SortOrder>,
    #[serde(default)]
    pub limit: Option<i64>,
    #[serde(default)]
    pub offset: Option<i64>,
}

// Generic field type definitions
#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    String,
    Number,
    DateTime,
    Boolean,
}

// Generic field structure
#[derive(Debug, Clone)]
pub struct Field {
    pub name: &'static str,
    pub field_type: FieldType,
}

// Relationship type definitions
#[derive(Debug, Clone)]
pub enum RelationKind {
    BelongsTo,
    HasMany,
    HasOne,
}

// Relationship structure
#[derive(Debug, Clone)]
pub struct Relation {
    pub kind: RelationKind,
    pub target: &'static str,
    pub foreign_key: &'static str,
}


// Table information for query building
#[derive(Debug, Clone)]
pub struct Table {
    pub name: &'static str,
    pub alias: String,
    pub fields: Vec<Field>,
    pub relations: HashMap<String, Relation>,
    pub primary_key: &'static str,
}

// Path information for traversing relationships
#[derive(Debug, Clone)]
pub struct ColumnPath {
    pub tables: Vec<Table>,
    pub joins: Vec<String>,
    pub column_name: String,
    pub field_type: FieldType,
}

// Registry to look up models by name
#[derive(Default)]
pub struct ModelRegistry {
    pub models: HashMap<&'static str, Table>,
    pub root_model: &'static str,
}

pub trait Relatable {
    fn get_relations() -> HashMap<String, Relation>;
    fn get_fields() -> Vec<Field>;
    fn get_table_name() -> &'static str;
    fn get_primary_key() -> &'static str;
    fn into_table() -> Table {
        Table {
            name: Self::get_table_name(),
            alias: Self::get_table_name().to_string(),
            fields: Self::get_fields(),
            relations: Self::get_relations(),
            primary_key: Self::get_primary_key(),
        }
    }
}

impl ModelRegistry {
    pub fn new(root_table: Table) -> Self {
        let mut models = HashMap::new();
        let root_model = root_table.name;
        models.insert(root_model, root_table);
        
        Self {
            models,
            root_model,
        }
    }

    pub fn add_table(&mut self, table: Table) {
        self.models.insert(table.name, table);
    }

    pub fn get_model(&self, model_name: &str) -> Option<&Table> {
        self.models.get(model_name)
    }

    pub fn get_root_model(&self) -> Option<&Table> {
        self.get_model(self.root_model)
    }
}