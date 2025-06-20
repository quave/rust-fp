extern crate proc_macro;

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput, Data, Fields, Type, Attribute};

/// A derive macro to automatically implement the Relatable trait for a struct.
/// 
/// This macro analyzes the struct fields and creates an implementation of the Relatable
/// trait that includes all fields with appropriate type information.
/// 
/// # Example
/// ```c
/// #[derive(Relatable)]
/// #[table_name("orders")]
/// #[import_path("processing::ui_model")]
/// #[relation(r#""transaction" => (RelationKind::BelongsTo, "transactions", "transaction_id")"#)]
/// #[relation(r#""items" => (RelationKind::HasMany, "order_items", "order_id")"#)]
/// pub struct DbOrder {
///     pub id: ModelId,
///     pub transaction_id: ModelId,
///     // ... other fields
/// }
/// ```
#[proc_macro_derive(Relatable, attributes(table_name, relation, import_path, primary_key))]
pub fn derive_relatable(input: TokenStream) -> TokenStream {
    let ast = parse_macro_input!(input as DeriveInput);
    let name = &ast.ident;
    
    // Get table name from attribute or use a default
    let table_name = find_attribute_value(&ast.attrs, "table_name")
        .unwrap_or_else(|| name.to_string().to_lowercase() + "s");
    
    // Get the module path to import from
    let module_path = find_attribute_value(&ast.attrs, "import_path")
        .unwrap_or_else(|| "processing::ui_model".to_string());

    // Get the primary key from attribute or use a default
    let primary_key = find_attribute_value(&ast.attrs, "primary_key")
        .unwrap_or_else(|| "id".to_string());
    
    // Extract fields from the struct
    let fields = match &ast.data {
        Data::Struct(data) => &data.fields,
        _ => panic!("Relatable can only be derived for structs"),
    };
    
    // Create field list
    let field_defs = extract_field_defs(fields, &module_path);
    
    // Parse relation attributes
    let relation_defs = parse_relation_attributes(&ast.attrs, &module_path);
    
    // Convert the module path string to a TokenStream
    let module_path_tokens = syn::parse_str::<syn::Path>(&module_path)
        .expect("Invalid module path");
    
    // Generate implementation
    let expanded = quote! {
        impl #module_path_tokens::Relatable for #name {
            fn get_relations() -> std::collections::HashMap<String, #module_path_tokens::Relation> {
                let mut relations = std::collections::HashMap::new();
                
                // Add relations from attributes
                #(#relation_defs)*
                
                relations
            }

            fn get_fields() -> Vec<#module_path_tokens::Field> {
                vec![
                    #(#field_defs),*
                ]
            }

            fn get_table_name() -> &'static str {
                #table_name
            }

            fn get_primary_key() -> &'static str {
                #primary_key
            }
        }
    };
    
    TokenStream::from(expanded)
}

/// A proc macro for creating a model registry with specified models.
/// 
/// This macro generates a function that creates a ModelRegistry instance,
/// sets the root model type, and registers all specified models.
/// 
/// # Example
/// ```c
/// create_registry_macro!("Transaction", DbOrder, DbOrderItem, DbCustomerData, DbBillingData);
/// ```
#[proc_macro]
pub fn get_registry_macro(input: TokenStream) -> TokenStream {
    let input_str = input.to_string();
    let parts: Vec<&str> = input_str.split(',').collect();
    
    // If there's only one part, it's just the model types, and the first one is the root type
    let model_types: Vec<&str> = parts.iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();
    
    // Generate code for each model registration
    let model_registrations = model_types.iter().map(|model| {
        let model = model.trim_start_matches('&').trim();
        format!("registry.register(\"{}\", {}::default());", model, model)
    }).collect::<Vec<String>>().join("\n            ");
    
    // Generate the registry instance
    let expanded = format!(
        r#"
            let mut registry = processing::ui_model::ModelRegistry::new();
            
            registry.set_root_model_type("{}");
            
            // Register all models
            {}
            
            registry
        "#,
        model_types[0],  // Use the first model as the root type
        model_registrations
    );
    
    expanded.parse().unwrap()
}

// Find the string value from an attribute like #[attr("value")]
fn find_attribute_value(attrs: &[Attribute], name: &str) -> Option<String> {
    for attr in attrs {
        if attr.path().is_ident(name) {
            if let Ok(expr) = attr.parse_args::<syn::LitStr>() {
                return Some(expr.value());
            }
        }
    }
    None
}

// Extract field definitions from struct fields
fn extract_field_defs(fields: &Fields, module_path: &str) -> Vec<proc_macro2::TokenStream> {
    // Convert the module path string to a TokenStream
    let module_path_tokens = syn::parse_str::<syn::Path>(module_path)
        .expect("Invalid module path");
    
    fields.iter().map(|f| {
        let name = f.ident.as_ref().unwrap().to_string();
        let field_type = determine_field_type(&f.ty);
        
        quote! {
            #module_path_tokens::Field {
                name: #name,
                field_type: #module_path_tokens::FieldType::#field_type
            }
        }
    }).collect()
}

// Determine FieldType from Rust type
fn determine_field_type(ty: &Type) -> proc_macro2::TokenStream {
    match ty {
        Type::Path(type_path) => {
            let type_name = &type_path.path.segments.last().unwrap().ident.to_string();
            match type_name.as_str() {
                "String" => quote! { String },
                "i32" | "i64" | "f32" | "f64" | "ModelId" => quote! { Number },
                "bool" => quote! { Boolean },
                "DateTime" => quote! { DateTime },
                _ => quote! { String }, // Default to String for unknown types
            }
        },
        _ => quote! { String }, // Default to String for complex types
    }
}

// Parse the relation attributes
fn parse_relation_attributes(attrs: &[Attribute], module_path: &str) -> Vec<proc_macro2::TokenStream> {
    // Convert the module path string to a TokenStream
    let module_path_tokens = syn::parse_str::<syn::Path>(module_path)
        .expect("Invalid module path");
    
    // Collect relation tokens
    let mut relation_tokens = Vec::new();
    
    for attr in attrs {
        if attr.path().is_ident("relation") {
            // Get the attribute as a string and process it
            match attr.meta.clone() {
                syn::Meta::List(list) => {
                    // Convert tokens to string for parsing
                    let tokens_str = list.tokens.to_string();
                    
                    // Extract relation information using regex patterns
                    // Format expected: r#""name" => (RelationKind::Type, "target", "foreign_key")"#
                    
                    // Split by "=>" to get the name and the rest
                    if let Some(arrow_pos) = tokens_str.find("=>") {
                        // Extract name part (before the =>)
                        let name_part = tokens_str[..arrow_pos].trim();
                        // Remove any quotes and 'r#' prefix
                        let name = name_part.trim_matches(|c| c == '"' || c == '\'' || c == 'r' || c == '#');
                        
                        // Extract the relation tuple part (after the =>)
                        let rel_part = tokens_str[arrow_pos + 2..].trim();
                        
                        // Check for tuple format (...)
                        if rel_part.starts_with('(') && rel_part.ends_with(')') {
                            // Extract content inside parentheses
                            let tuple_content = &rel_part[1..rel_part.len() - 1];
                            
                            // Split by commas to get individual parts
                            let parts: Vec<&str> = tuple_content.split(',').map(|s| s.trim()).collect();
                            
                            if parts.len() >= 3 {
                                // First part is the relation kind (RelationKind::Type)
                                let kind_str = parts[0];
                                let kind_parts: Vec<&str> = kind_str.split("::").collect();
                                if kind_parts.len() != 2 {
                                    continue; // Invalid format
                                }
                                let kind_enum = kind_parts[0]; // Should be "RelationKind"
                                let kind_variant = kind_parts[1]; // BelongsTo, HasMany, or HasOne
                                
                                // Second part is the target table name
                                let target = parts[1].trim_matches('"');
                                
                                // Third part is the foreign key
                                let foreign_key = parts[2].trim_matches('"');
                                
                                // Create token for relation
                                let relation_token = quote! {
                                    relations.insert(
                                        #name.to_string(),
                                        #module_path_tokens::Relation {
                                            kind: #module_path_tokens::#kind_enum::#kind_variant,
                                            target: #target,
                                            foreign_key: #foreign_key,
                                        }
                                    );
                                };
                                
                                relation_tokens.push(relation_token);
                            }
                        }
                    }
                },
                _ => continue,
            }
        }
    }
    
    relation_tokens
} 