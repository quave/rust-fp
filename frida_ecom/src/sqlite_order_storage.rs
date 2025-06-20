use crate::{ecom_db_model::*, ecom_import_model::*};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use frida_core::{
    model::{Feature, FeatureValue, Processible, ScorerResult},
    storage::Storage,
};
use log::{debug, error, info};
use serde_json::{json, Value};
use sqlx::SqliteConnection;
use std::{error::Error, marker::PhantomData};

pub struct SqliteOrderStorage {
    pool: sqlx::SqlitePool,
    _phantom: PhantomData<Order>,
}

#[async_trait]
impl Storage<ImportOrder, Order> for SqliteOrderStorage {
    async fn new(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let pool = sqlx::SqlitePool::connect(database_url).await?;
        Ok(Self {
            pool,
            _phantom: PhantomData,
        })
    }

    async fn initialize_schema(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let init_sql = include_str!("../../resources/init.sql");
        sqlx::query(init_sql).execute(&self.pool).await?;
        Ok(())
    }

    async fn save_transaction(
        &self,
        order: &ImportOrder,
    ) -> Result<<Order as Processible>::Id, Box<dyn Error + Send + Sync>> {
        debug!(
            "Starting to save transaction for order_number: {}",
            order.order_number
        );

        // Start a transaction
        let mut tx = self.pool.begin().await?;

        // 1. First, insert the main order record
        let order_id = self.save_order(order, &mut *tx).await?;

        // 2. Insert customer data
        let _ = self
            .save_customer(order_id, &order.customer, &mut tx)
            .await?;
        // Add more debug statements for other operations...
        // 3. Insert order items
        let _ = self
            .save_order_items(order_id, &order.items, &mut tx)
            .await?;

        // 4. Insert billing data
        let _ = self.save_billing(order_id, &order.billing, &mut tx).await?;
        // Commit all changes
        debug!(
            "Committing transaction for order_id: {}, order_number: {}",
            order_id, order.order_number
        );
        match tx.commit().await {
            Ok(_) => debug!("Successfully committed transaction"),
            Err(e) => {
                error!("Failed to commit transaction: {}", e);
                return Err(e.into());
            }
        }

        info!("Successfully saved order: {}", order.order_number);
        Ok(order_id)
    }

    async fn save_features(
        &self,
        order_id: &i64,
        features: &[Feature],
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut tx = self.pool.begin().await?;

        for feature in features {
            sqlx::query(
                r#"
                INSERT INTO features (
                    order_id, feature_name, feature_value_type, feature_value
                ) VALUES (?, ?, ?, ?)
                "#,
            )
            .bind(order_id)
            .bind(&feature.name)
            .bind(get_feature_type(&feature.value))
            .bind(serialize_feature_value(&feature.value)?)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    async fn save_scores(
        &self,
        transaction_id: &i64,
        scores: &Vec<ScorerResult>,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let mut tx = self.pool.begin().await?;

        // Save individual triggered rules
        for score in scores {
            match sqlx::query!(
                r#"
                INSERT INTO triggered_rules (order_id, rule_name, rule_score) 
                VALUES (?, ?, ?)
                "#,
                transaction_id,
                score.name,
                score.score
            )
            .execute(&mut *tx)
            .await
            {
                Ok(_) => {
                    debug!("Successfully inserted customer data");
                }
                Err(e) => {
                    error!("Failed to insert customer data: {}", e);
                }
            }
        }

        tx.commit().await?;
        Ok(())
    }

    async fn get_transaction(&self, id: &i64) -> Result<Order, Box<dyn Error + Send + Sync>> {
        let mut tx = self.pool.begin().await?;

        // Get main order data
        let order = sqlx::query!(
            r#"
            SELECT 
                id, 
                order_number, 
                delivery_type, 
                delivery_details, 
                created_at as "created_at: DateTime<Utc>"
            FROM orders
            WHERE id = ?
            "#,
            id
        )
        .fetch_one(&mut *tx)
        .await
        .map(|rec| DbOrder {
            id: rec.id,
            order_number: rec.order_number.unwrap(),
            delivery_type: rec.delivery_type,
            delivery_details: rec.delivery_details,
            created_at: rec.created_at.expect("must be present"),
        })?;

        // Get order items
        let items = sqlx::query!(
            r#"
            SELECT id, order_id, name, category, price, created_at as "created_at: DateTime<Utc>"
            FROM order_items
            WHERE order_id = ?
            "#,
            id
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|row| DbOrderItem {
            id: row.id.unwrap(),
            order_id: row.order_id,
            name: row.name,
            category: row.category,
            price: row.price,
            created_at: row.created_at,
        })
        .collect();

        // Get customer data
        let customer: DbCustomerData = sqlx::query!(
            r#"
            SELECT id, order_id, name, email, created_at as "created_at: DateTime<Utc>"
            FROM customers
            WHERE order_id = ?
            "#,
            id
        )
        .fetch_one(&mut *tx)
        .await
        .map(|rec| DbCustomerData {
            id: rec.id.expect("must be present"),
            order_id: rec.order_id,
            name: rec.name,
            email: rec.email,
            created_at: rec.created_at,
        })?;

        // Get billing data
        let billing: DbBillingData = sqlx::query!(
            r#"
            SELECT id, order_id, payment_type, payment_details, billing_address, created_at as "created_at: DateTime<Utc>"
            FROM billing_data
            WHERE order_id = ?
            "#,
            id
        )
        .fetch_one(&mut *tx)
        .await
        .map(|rec| DbBillingData {
            id: rec.id.unwrap(),
            order_id: rec.order_id,
            payment_type: rec.payment_type,
            payment_details: rec.payment_details,
            billing_address: rec.billing_address,
            created_at: rec.created_at,
        })?;

        tx.commit().await?;

        Ok(Order {
            order: order,
            items: items,
            customer: customer,
            billing: billing,
        })
    }
}

impl SqliteOrderStorage {
    async fn save_order(
        &self,
        order: &ImportOrder,
        tx: &mut SqliteConnection,
    ) -> Result<i64, Box<dyn Error + Send + Sync>> {
        debug!(
            "Inserting main order record for order_id: {}",
            order.order_number
        );
        match sqlx::query!(
            r#"
            INSERT INTO orders (
                order_number, delivery_type, delivery_details
            ) VALUES (?, ?, ?)
            RETURNING id
            "#,
            order.order_number,
            order.delivery_type,
            order.delivery_details
        )
        .fetch_one(tx)
        .await
        {
            Ok(record) => {
                debug!("Successfully inserted main order record");
                Ok(record.id)
            }
            Err(e) => {
                error!("Failed to insert main order record: {}", e);
                return Err(e.into());
            }
        }
    }

    async fn save_customer(
        &self,
        order_id: i64,
        customer: &ImportCustomerData,
        tx: &mut SqliteConnection,
    ) -> Result<i64, Box<dyn Error + Send + Sync>> {
        debug!("Inserting customer data for order_id: {}", order_id);
        match sqlx::query!(
            r#"
            INSERT INTO customers (
                order_id, name, email
            ) VALUES (?, ?, ?)
            RETURNING id
            "#,
            order_id,
            customer.name,
            customer.email
        )
        .fetch_one(&mut *tx)
        .await
        {
            Ok(rec) => {
                debug!("Successfully inserted customer data");
                Ok(rec.id.unwrap())
            }
            Err(e) => {
                error!("Failed to insert customer data: {}", e);
                return Err(e.into());
            }
        }
    }

    async fn save_order_items(
        &self,
        order_id: i64,
        items: &[ImportOrderItem],
        tx: &mut SqliteConnection,
    ) -> Result<Vec<i64>, Box<dyn Error + Send + Sync>> {
        debug!(
            "Inserting {} order items for order_id: {}",
            items.len(),
            order_id
        );

        let mut item_ids = Vec::new();
        for item in items {
            match sqlx::query!(
                r#"
                INSERT INTO order_items (
                    order_id, name, category, price
                ) VALUES (?, ?, ?, ?)
                RETURNING id
                "#,
                order_id,
                item.name,
                item.category,
                item.price
            )
            .fetch_one(&mut *tx)
            .await
            {
                Ok(rec) => {
                    item_ids.push(rec.id.unwrap());
                }
                Err(e) => {
                    error!("Failed to insert order item: {}", e);
                    return Err(e.into());
                }
            }
        }

        Ok(item_ids)
    }

    async fn save_billing(
        &self,
        order_id: i64,
        billing: &ImportBillingData,
        tx: &mut SqliteConnection,
    ) -> Result<i64, Box<dyn Error + Send + Sync>> {
        debug!("Inserting billing info for order_id: {}", order_id);

        match sqlx::query!(
            r#"
            INSERT INTO billing_data (
                order_id, payment_type, payment_details, billing_address
            ) VALUES (?, ?, ?, ?)
            RETURNING id
            "#,
            order_id,
            billing.payment_type,
            billing.payment_details,
            billing.billing_address
        )
        .fetch_one(tx)
        .await
        {
            Ok(rec) => {
                debug!("Successfully inserted billing info");
                Ok(rec.id.unwrap())
            }
            Err(e) => {
                error!("Failed to insert billing info: {}", e);
                Err(e.into())
            }
        }
    }

    pub async fn get_features(
        &self,
        order_id: &str,
    ) -> Result<Vec<Feature>, Box<dyn Error + Send + Sync>> {
        let features = sqlx::query!(
            r#"
            SELECT feature_name, feature_value_type, feature_value
            FROM features
            WHERE order_id = ?
            "#,
            order_id
        )
        .fetch_all(&self.pool)
        .await?
        .into_iter()
        .map(|row| Feature {
            name: row.feature_name,
            value: deserialize_feature_value(&row.feature_value).unwrap(), // You'll need to implement this
        })
        .collect();

        Ok(features)
    }
}

// Helper functions for feature serialization
fn get_feature_type(value: &FeatureValue) -> &'static str {
    match value {
        FeatureValue::Int(_) => "INT",
        FeatureValue::Double(_) => "DOUBLE",
        FeatureValue::String(_) => "STRING",
        FeatureValue::Bool(_) => "BOOL",
        FeatureValue::DateTime(_) => "DATETIME",
        FeatureValue::IntList(_) => "INT_LIST",
        FeatureValue::DoubleList(_) => "DOUBLE_LIST",
        FeatureValue::StringList(_) => "STRING_LIST",
        FeatureValue::BoolList(_) => "BOOL_LIST",
    }
}

fn serialize_feature_value(value: &FeatureValue) -> Result<String, serde_json::Error> {
    Ok(match value {
        FeatureValue::Int(v) => json!({ "type": "int", "value": v }).to_string(),
        FeatureValue::Double(v) => json!({ "type": "double", "value": v }).to_string(),
        FeatureValue::String(v) => json!({ "type": "string", "value": v }).to_string(),
        FeatureValue::Bool(v) => json!({ "type": "bool", "value": v }).to_string(),
        FeatureValue::DateTime(v) => {
            json!({ "type": "datetime", "value": v.to_rfc3339() }).to_string()
        }
        FeatureValue::IntList(v) => json!({ "type": "int_list", "value": v }).to_string(),
        FeatureValue::DoubleList(v) => json!({ "type": "double_list", "value": v }).to_string(),
        FeatureValue::StringList(v) => json!({ "type": "string_list", "value": v }).to_string(),
        FeatureValue::BoolList(v) => json!({ "type": "bool_list", "value": v }).to_string(),
    })
}

fn deserialize_feature_value(json_str: &str) -> Result<FeatureValue, Box<dyn Error + Send + Sync>> {
    let value: Value = serde_json::from_str(json_str)?;

    let type_str = value["type"]
        .as_str()
        .ok_or("Missing or invalid 'type' field")?;

    match type_str {
        "int" => Ok(FeatureValue::Int(
            value["value"].as_i64().ok_or("Invalid integer value")?,
        )),

        "double" => Ok(FeatureValue::Double(
            value["value"].as_f64().ok_or("Invalid double value")?,
        )),

        "string" => Ok(FeatureValue::String(
            value["value"]
                .as_str()
                .ok_or("Invalid string value")?
                .to_string(),
        )),

        "bool" => Ok(FeatureValue::Bool(
            value["value"].as_bool().ok_or("Invalid boolean value")?,
        )),

        "datetime" => {
            let datetime_str = value["value"].as_str().ok_or("Invalid datetime string")?;
            Ok(FeatureValue::DateTime(
                DateTime::parse_from_rfc3339(datetime_str)?.with_timezone(&Utc),
            ))
        }

        "int_list" => {
            let values = value["value"].as_array().ok_or("Invalid array value")?;
            let mut list = Vec::new();
            for v in values {
                list.push(v.as_i64().ok_or("Invalid integer in array")?);
            }
            Ok(FeatureValue::IntList(list))
        }

        "double_list" => {
            let values = value["value"].as_array().ok_or("Invalid array value")?;
            let mut list = Vec::new();
            for v in values {
                list.push(v.as_f64().ok_or("Invalid double in array")?);
            }
            Ok(FeatureValue::DoubleList(list))
        }

        "string_list" => {
            let values = value["value"].as_array().ok_or("Invalid array value")?;
            let mut list = Vec::new();
            for v in values {
                list.push(v.as_str().ok_or("Invalid string in array")?.to_string());
            }
            Ok(FeatureValue::StringList(list))
        }

        "bool_list" => {
            let values = value["value"].as_array().ok_or("Invalid array value")?;
            let mut list = Vec::new();
            for v in values {
                list.push(v.as_bool().ok_or("Invalid boolean in array")?);
            }
            Ok(FeatureValue::BoolList(list))
        }

        _ => Err("Unknown feature value type".into()),
    }
}
