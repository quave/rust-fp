use crate::model::ModelRegistryProvider;
use crate::ui_model::{
    RelationKind, FieldType, FilterRequest, FilterGroup,
    SortOrder, SortDirection, LogicalOperator, Operator, FilterValue,
    ColumnPath, Table, Relation
};
use sqlx::postgres::PgArguments;
use sqlx::Arguments;
use std::collections::{HashMap, HashSet};
use std::marker::PhantomData;

/// Helper function to build a query using the SQL query builder
/// This is provided for backward compatibility
pub fn build_query<T: ModelRegistryProvider>(
    filter_request: &FilterRequest
) -> Result<(String, PgArguments), String> {
    SqlQueryBuilder::<T>::build_query(filter_request)
}

pub struct SqlQueryBuilder<T: ModelRegistryProvider> {
    _marker: PhantomData<T>,
}

impl<T: ModelRegistryProvider> SqlQueryBuilder<T> {
    /// Build SQL query from filter request using PostgreSQL
    pub fn build_query(
        filter_request: &FilterRequest
    ) -> Result<(String, PgArguments), String> {
        let registry = T::get_registry();
        // Create a base query from the root model
        let root_model = registry.get_root_model()
            .ok_or_else(|| "Root model not found in registry".to_string())?;
        
        // Changed: Select only the primary key (id) instead of all columns (*)
        let base_query = format!("SELECT {}.{} FROM {} {}", 
            root_model.alias, 
            root_model.primary_key, 
            root_model.name, 
            root_model.alias);
        
        // Find all columns that have filter conditions to determine join types
        let filtered_columns = if let Some(filter_group) = &filter_request.filter {
            Self::extract_filtered_columns(filter_group)
        } else {
            HashSet::new()
        };
        
        // Process the joins and collect column paths
        let (query_with_joins, column_paths) = Self::process_joins(filter_request, base_query, &filtered_columns)?;

        // Prepare arguments
        let mut args = PgArguments::default();
        
        // Add where clause if filter group exists
        let mut final_query = query_with_joins;
        
        if let Some(filter_group) = &filter_request.filter {
            let where_clause = Self::build_where_clause(filter_group, &column_paths, &mut args, 1);
            if !where_clause.is_empty() {
                final_query = format!("{} WHERE {}", final_query, where_clause);
            }
        }
        
        // Add order by clause
        let order_by = Self::build_order_by(&filter_request.sort, &column_paths);
        if !order_by.is_empty() {
            final_query = format!("{} {}", final_query, order_by);
        }
        
        // Add limit if present
        if let Some(limit) = filter_request.limit {
            final_query = format!("{} LIMIT {}", final_query, limit);
        }
        
        // Add offset if present
        if let Some(offset) = filter_request.offset {
            final_query = format!("{} OFFSET {}", final_query, offset);
        }
        
        // Return the query and arguments
        Ok((final_query, args))
    }

    /// Extract all columns that have filter conditions
    pub fn extract_filtered_columns(filter_group: &FilterGroup) -> HashSet<String> {
        let mut filtered_columns = HashSet::new();
        
        // Add all columns from this group
        for condition in &filter_group.conditions {
            // Only use inner joins for non-IS NULL operators
            // IS NOT NULL should use INNER JOIN as it filters out NULL values
            if !matches!(condition.operator, Operator::IsNull) {
                filtered_columns.insert(condition.column.clone());
            }
        }
        
        // Process nested groups
        for group in &filter_group.groups {
            filtered_columns.extend(Self::extract_filtered_columns(group));
        }
        
        filtered_columns
    }
    
    /// Determine if a column should use an INNER JOIN based on the filter condition
    pub fn should_use_inner_join(column: &str, filtered_columns: &HashSet<String>) -> bool {
        println!("Checking if '{}' should use INNER JOIN", column);
        println!("Filtered columns: {:?}", filtered_columns);
        
        // Direct match - the column itself is used in a filter
        if filtered_columns.contains(column) {
            println!("Column is directly filtered, using INNER JOIN");
            return true;
        }
        
        // Now check if this column is a relation path (like "customer") that's used as a prefix
        // for any filtered columns (like "customer.name")
        for filtered_column in filtered_columns {
            // Check if any filtered column starts with this column as a prefix
            if filtered_column.starts_with(&format!("{}.", column)) {
                println!("Column is a prefix for filtered column '{}', using INNER JOIN", filtered_column);
                return true;
            }
            
            // Check if this is a parent relation of a filtered column
            let parts: Vec<&str> = filtered_column.split('.').collect();
            if parts.len() > 1 && parts[0] == column {
                println!("Column is a parent relation of filtered column '{}', using INNER JOIN", filtered_column);
                return true;
            }
        }
        
        println!("No related filtered columns found, using LEFT JOIN");
        false
    }

    /// Process all necessary joins based on filter_request
    pub fn process_joins(
        filter_request: &FilterRequest,
        base_query: String,
        filtered_columns: &HashSet<String>
    ) -> Result<(String, HashMap<String, ColumnPath>), String> {
        let mut column_paths = HashMap::new();
        let mut joins = Vec::new();
        
        let root_model = T::get_registry().get_root_model()
            .ok_or_else(|| "Root model must be registered".to_string())?;
        
        let mut tables = vec![root_model.clone()];
            
        // Process columns from filter conditions
        if let Some(filter_group) = &filter_request.filter {
            Self::extract_filter_columns(filter_group, &mut tables, &mut joins, &mut column_paths, filtered_columns)
                .map_err(|e| format!("Error extracting filter columns: {}", e))?;
        }
        
        // Process columns from sort orders
        for sort in &filter_request.sort {
            Self::process_column_path(&sort.column, &mut tables, &mut joins, &mut column_paths, filtered_columns)
                .map_err(|e| format!("Error processing sort column: {}", e))?;
        }
        
        // Build the query with joins
        let mut query = base_query;
        for join in joins {
            query = format!("{} {}", query, join);
        }
        
        Ok((query, column_paths))
    }

    /// Process a column path like "customer.name" to determine necessary joins
    pub fn process_column_path(
        path: &str, 
        tables: &mut Vec<Table>,
        joins: &mut Vec<String>,
        column_paths: &mut HashMap<String, ColumnPath>,
        filtered_columns: &HashSet<String>
    ) -> Result<(), String> {
        // Skip if already processed
        if column_paths.contains_key(path) {
            return Ok(());
        }
        
        let parts: Vec<&str> = path.split('.').collect();
        
        if parts.len() == 1 {
            // Simple field on root table
            return Self::process_simple_field(path, parts[0], tables, column_paths);
        }
        
        // Field on related table - process the relation path
        Self::process_relation_path(path, &parts, tables, joins, column_paths, filtered_columns)
    }
    
    /// Process a simple field on the root table
    fn process_simple_field(
        path: &str,
        field_name: &str,
        tables: &[Table],
        column_paths: &mut HashMap<String, ColumnPath>
    ) -> Result<(), String> {
        let registry = T::get_registry();
        let root_model = registry.get_root_model()
            .ok_or_else(|| "Root model must be registered".to_string())?;
            
        // Find field type from model definition
        let field_type = root_model.fields
            .iter()
            .find(|f| f.name == field_name)
            .map(|f| f.field_type.clone())
            .unwrap_or(FieldType::String);
            
        column_paths.insert(
            path.to_string(),
            ColumnPath {
                tables: tables.to_vec(),
                joins: Vec::new(),
                column_name: field_name.to_string(),
                field_type,
            },
        );
        Ok(())
    }
    
    /// Process a path that involves relationships between tables
    fn process_relation_path(
        path: &str,
        parts: &[&str],
        tables: &mut Vec<Table>,
        joins: &mut Vec<String>,
        column_paths: &mut HashMap<String, ColumnPath>,
        filtered_columns: &HashSet<String>
    ) -> Result<(), String> {
        let registry = T::get_registry();
        let mut current_model_name = registry.root_model;
        let mut current_table_info = tables[0].clone();
        let mut path_tables = vec![tables[0].clone()];
        let mut path_joins = Vec::new();
        
        // Process each part of the path (except the last one which is a field)
        for (i, part) in parts.iter().enumerate() {
            if i == parts.len() - 1 {
                // This is the actual field name (last part), not a relation
                let current_model = registry.get_model(current_model_name)
                    .ok_or_else(|| format!("Model {} must be registered", current_model_name))?;
                    
                let field_type = current_model.fields
                    .iter()
                    .find(|f| f.name == *part)
                    .map(|f| f.field_type.clone())
                    .unwrap_or(FieldType::String);
                    
                column_paths.insert(
                    path.to_string(),
                    ColumnPath {
                        tables: path_tables,
                        joins: path_joins,
                        column_name: part.to_string(),
                        field_type,
                    },
                );
                break;
            }
            
            // Get relation info
            let (relation, target_model) = Self::get_relation_info(current_model_name, part)?;
            
            // Build the relation path up to this point to check if it's filtered
            let relation_path = if i == 0 {
                part.to_string()
            } else {
                let mut prev_parts = parts[..i].to_vec();
                prev_parts.push(part);
                prev_parts.join(".")
            };
            
            // Determine join type based on whether the column is filtered
            let use_inner_join = Self::should_use_inner_join(&relation_path, filtered_columns);
            
            // Create join SQL
            let (join_sql, join_alias) = Self::create_join_sql(
                &current_table_info, 
                &target_model, 
                &relation, 
                joins.len(),
                use_inner_join
            );
            
            // Only add the join if it's not already present
            if !joins.contains(&join_sql) {
                joins.push(join_sql.clone());
            }
            path_joins.push(join_sql.clone());
            
            // Update current state for next iteration
            current_model_name = relation.target;
            let current_table = Table {
                name: target_model.name,
                alias: join_alias,
                fields: target_model.fields.clone(),
                relations: target_model.relations.clone(),
                primary_key: target_model.primary_key,
            };
            path_tables.push(current_table.clone());
            current_table_info = current_table;
        }
        Ok(())
    }
    
    /// Get relation and target model information
    fn get_relation_info(
        model_name: &str, 
        relation_name: &str
    ) -> Result<(Relation, Table), String> {
        let registry = T::get_registry();
        
        // Get current model and its relations
        let current_model = registry.get_model(model_name)
            .ok_or_else(|| format!("Model {} must be registered", model_name))?;
            
        let relations = current_model.relations.clone();
        let relation = relations.get(relation_name)
            .ok_or_else(|| format!("Relation {} not found on model {}", relation_name, model_name))?;
            
        // Get target model info
        let target_model_name = relation.target;
        let target_model = registry.get_model(target_model_name)
            .ok_or_else(|| format!("Target model {} must be registered", target_model_name))?;
            
        Ok((relation.clone(), target_model.clone()))
    }
    
    /// Create SQL JOIN statement for a relation
    fn create_join_sql(
        current_table: &Table,
        target_model: &Table,
        relation: &Relation,
        join_index: usize,
        use_inner_join: bool
    ) -> (String, String) {
        let target_table = target_model.name;
        
        // Create a deterministic alias based on the first letter of the table and the join index
        let first_char = if !target_table.is_empty() {
            target_table.chars().next().unwrap().to_ascii_lowercase()
        } else {
            't'
        };
        
        // Generate aliases in the format "c_1", "o_2", etc. to match the test expectations
        // Adding 1 to the join_index to match the original test expectations (which used 1-based indices)
        let join_alias = format!("{}_{}", first_char, join_index + 1);
        
        // Choose join type based on whether the column is filtered
        let join_type = if use_inner_join { "INNER JOIN" } else { "LEFT JOIN" };
        
        let join_sql = match relation.kind {
            RelationKind::HasOne | RelationKind::HasMany => {
                format!(
                    "{} {} {} ON {}.{} = {}.{}",
                    join_type,
                    target_table,
                    join_alias,
                    join_alias,
                    relation.foreign_key,
                    current_table.alias,
                    "id"
                )
            }
            RelationKind::BelongsTo => {
                format!(
                    "{} {} {} ON {}.{} = {}.{}",
                    join_type,
                    target_table,
                    join_alias,
                    current_table.alias,
                    relation.foreign_key,
                    join_alias,
                    "id"
                )
            }
        };
        
        (join_sql, join_alias)
    }

    /// Extract all columns referenced in filter conditions
    pub fn extract_filter_columns(
        filter_group: &FilterGroup, 
        tables: &mut Vec<Table>,
        joins: &mut Vec<String>,
        column_paths: &mut HashMap<String, ColumnPath>,
        filtered_columns: &HashSet<String>
    ) -> Result<(), String> {
        // Process conditions in this group
        for condition in &filter_group.conditions {
            Self::process_column_path(&condition.column, tables, joins, column_paths, filtered_columns)
                .map_err(|e| format!("Error processing condition column: {}", e))?;
        }
        
        // Process nested groups
        for group in &filter_group.groups {
            Self::extract_filter_columns(group, tables, joins, column_paths, filtered_columns)
                .map_err(|e| format!("Error processing nested group: {}", e))?;
        }
        Ok(())
    }

    /// Build the WHERE clause from filter groups
    pub fn build_where_clause(
        filter_group: &FilterGroup, 
        column_paths: &HashMap<String, ColumnPath>,
        args: &mut PgArguments,
        start_param_index: usize
    ) -> String {
        let mut param_index = start_param_index;
        let mut conditions = Vec::new();
        
        // Process individual conditions
        for condition in &filter_group.conditions {
            if let Some(column_path) = column_paths.get(&condition.column) {
                let last_table = &column_path.tables.last().unwrap();
                let column_expr = format!("{}.{}", last_table.alias, column_path.column_name);
                
                let condition_sql = match &condition.value {
                    FilterValue::String(val) => {
                        // Special case for IS NULL/IS NOT NULL operators which don't need parameters
                        if matches!(condition.operator, Operator::IsNull) {
                            format!("{} IS NULL", column_expr)
                        } else if matches!(condition.operator, Operator::IsNotNull) {
                            format!("{} IS NOT NULL", column_expr)
                        } else {
                            // Regular string parameter
                            let _ = args.add(val.clone());
                            format!("{} {} ${}", column_expr, condition.operator.to_sql(), param_index)
                        }
                    },
                    FilterValue::Number(val) => {
                        let _ = args.add(*val);
                        format!("{} {} ${}", column_expr, condition.operator.to_sql(), param_index)
                    },
                    FilterValue::Boolean(val) => {
                        let _ = args.add(*val);
                        format!("{} {} ${}", column_expr, condition.operator.to_sql(), param_index)
                    },
                    FilterValue::StringArray(vals) => {
                        if matches!(condition.operator, Operator::In | Operator::NotIn) {
                            let placeholders: Vec<String> = (0..vals.len())
                                .map(|i| {
                                    let _ = args.add(vals[i].clone());
                                    format!("${}", param_index + i)
                                })
                                .collect();
                            param_index += vals.len() - 1;
                            format!("{} {} ({})", column_expr, condition.operator.to_sql(), placeholders.join(", "))
                        } else {
                            "1=1".to_string() // Fallback for incorrect operator
                        }
                    },
                    FilterValue::NumberArray(vals) => {
                        if matches!(condition.operator, Operator::In | Operator::NotIn) {
                            let placeholders: Vec<String> = (0..vals.len())
                                .map(|i| {
                                    let _ = args.add(vals[i]);
                                    format!("${}", param_index + i)
                                })
                                .collect();
                            param_index += vals.len() - 1;
                            format!("{} {} ({})", column_expr, condition.operator.to_sql(), placeholders.join(", "))
                        } else {
                            "1=1".to_string() // Fallback for incorrect operator
                        }
                    },
                    FilterValue::Range { min, max } => {
                        if matches!(condition.operator, Operator::Between) {
                            let _ = args.add(*min);
                            let _ = args.add(*max);
                            let result = format!("{} BETWEEN ${} AND ${}", column_expr, param_index, param_index + 1);
                            param_index += 1; // Increment for the second parameter
                            result
                        } else {
                            "1=1".to_string() // Fallback for incorrect operator
                        }
                    },
                };
                conditions.push(condition_sql);
                
                // Only increment the parameter index if we added a parameter
                if !matches!(condition.operator, Operator::IsNull | Operator::IsNotNull) {
                    param_index += 1;
                }
            }
        }
        
        // Process nested groups
        for group in &filter_group.groups {
            let nested_condition = Self::build_where_clause(group, column_paths, args, param_index);
            conditions.push(format!("({})", nested_condition));
            
            // Count parameters used in the nested group to update param_index
            let params_used = Self::count_parameters(group, column_paths);
            param_index += params_used;
        }
        
        // Join all conditions with the logical operator
        let op_str = match filter_group.operator {
            LogicalOperator::And => "AND",
            LogicalOperator::Or => "OR",
        };
        
        if conditions.is_empty() {
            "1=1".to_string() // Default condition if no conditions are present
        } else {
            conditions.join(&format!(" {} ", op_str))
        }
    }

    /// Count the number of parameters used in a filter group
    fn count_parameters(
        filter_group: &FilterGroup,
        column_paths: &HashMap<String, ColumnPath>
    ) -> usize {
        let mut count = 0;
        
        // Count parameters for conditions
        for condition in &filter_group.conditions {
            if let Some(_) = column_paths.get(&condition.column) {
                // For IS NULL/IS NOT NULL operators, no parameters are needed
                if matches!(condition.operator, Operator::IsNull | Operator::IsNotNull) {
                    continue;
                }
                
                match &condition.value {
                    FilterValue::StringArray(vals) => {
                        if matches!(condition.operator, Operator::In | Operator::NotIn) {
                            count += vals.len();
                        } else {
                            count += 1;
                        }
                    },
                    FilterValue::NumberArray(vals) => {
                        if matches!(condition.operator, Operator::In | Operator::NotIn) {
                            count += vals.len();
                        } else {
                            count += 1;
                        }
                    },
                    FilterValue::Range { .. } => {
                        if matches!(condition.operator, Operator::Between) {
                            count += 2;
                        } else {
                            count += 1;
                        }
                    },
                    _ => count += 1,
                }
            }
        }
        
        // Count parameters for nested groups
        for group in &filter_group.groups {
            count += Self::count_parameters(group, column_paths);
        }
        
        count
    }

    /// Build ORDER BY clause
    pub fn build_order_by(
        sorts: &[SortOrder],
        column_paths: &HashMap<String, ColumnPath>
    ) -> String {
        if sorts.is_empty() {
            return String::new();
        }
        
        let sort_clauses: Vec<String> = sorts
            .iter()
            .filter_map(|sort| {
                column_paths.get(&sort.column).map(|path| {
                    let last_table = path.tables.last().unwrap();
                    let direction = match sort.direction {
                        SortDirection::Ascending => "ASC",
                        SortDirection::Descending => "DESC",
                    };
                    format!("{}.{} {}", last_table.alias, path.column_name, direction)
                })
            })
            .collect();
        
        if sort_clauses.is_empty() {
            String::new()
        } else {
            format!("ORDER BY {}", sort_clauses.join(", "))
        }
    } 
}
