use crate::{ecom_db_model::*, ecom_import_model::*};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use frida_core::{
    model::ModelId,
    storage::{ImportableStorage, ProcessibleStorage},
};
use log::{debug, error, info};
use sqlx::PgConnection;
use std::{error::Error, marker::PhantomData};

pub struct EcomOrderStorage {
    pub pool: sqlx::PgPool,
    _phantom: PhantomData<Order>,
}

impl EcomOrderStorage {
    pub async fn new(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let pool = sqlx::PgPool::connect(database_url).await?;
        Ok(Self {
            pool,
            _phantom: PhantomData,
        })
    }

    // Database insertion methods
    pub async fn insert_order(
        &self,
        tx: &mut PgConnection,
        transaction_id: ModelId,
        order_number: &str,
        delivery_type: &str,
        delivery_details: &str,
        created_at: DateTime<Utc>,
    ) -> Result<i64, sqlx::Error> {
        let order = sqlx::query!(
            r#"
            INSERT INTO orders (transaction_id, order_number, delivery_type, delivery_details, created_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
            transaction_id,
            order_number,
            delivery_type,
            delivery_details,
            created_at.naive_utc()
        )
        .fetch_one(tx)
        .await?;
        Ok(order.id)
    }

    pub async fn insert_order_item(
        &self,
        tx: &mut PgConnection,
        order_id: i64,
        name: &str,
        category: &str,
        price: f32,
        created_at: DateTime<Utc>,
    ) -> Result<i64, sqlx::Error> {
        let item = sqlx::query!(
            r#"
            INSERT INTO order_items (order_id, name, category, price, created_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
            order_id,
            name,
            category,
            price,
            created_at.naive_utc()
        )
        .fetch_one(tx)
        .await?;
        Ok(item.id)
    }

    pub async fn insert_customer(
        &self,
        tx: &mut PgConnection,
        order_id: i64,
        name: &str,
        email: &str,
        created_at: DateTime<Utc>,
    ) -> Result<i64, sqlx::Error> {
        let customer = sqlx::query!(
            r#"
            INSERT INTO customers (order_id, name, email, created_at)
            VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
            order_id,
            name,
            email,
            created_at.naive_utc()
        )
        .fetch_one(tx)
        .await?;
        Ok(customer.id)
    }

    pub async fn insert_billing(
        &self,
        tx: &mut PgConnection,
        order_id: i64,
        payment_type: &str,
        payment_details: &str,
        billing_address: &str,
        created_at: DateTime<Utc>,
    ) -> Result<i64, sqlx::Error> {
        let billing = sqlx::query!(
            r#"
            INSERT INTO billing_data (order_id, payment_type, payment_details, billing_address, created_at)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id
            "#,
            order_id,
            payment_type,
            payment_details,
            billing_address,
            created_at.naive_utc()
        )
        .fetch_one(tx)
        .await?;
        Ok(billing.id)
    }
}

#[async_trait]
impl ImportableStorage<ImportOrder> for EcomOrderStorage {

    async fn save_transaction(
        &self,
        order: &ImportOrder,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        debug!(
            "Starting to save transaction for order_number: {}",
            order.order_number
        );



        // Start a transaction
        let mut tx = self.pool.begin().await?;

        let transaction_id = self.save_db_transaction(&mut *tx).await?;

        // 1. First, insert the main order record
        let order_id = self.save_order(transaction_id, order, &mut *tx).await?;

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
        Ok(transaction_id)
    }
}

#[async_trait]
impl ProcessibleStorage<Order> for EcomOrderStorage {
    async fn get_processible(&self, id: ModelId) -> Result<Order, Box<dyn Error + Send + Sync>> {
        let mut tx = self.pool.begin().await?;

        debug!("Getting order data for transaction_id: {}", id);
        // Get main order data
        let order = sqlx::query!(
            r#"
            SELECT 
                id, 
                transaction_id,
                order_number, 
                delivery_type, 
                delivery_details, 
                created_at as "created_at: DateTime<Utc>"
            FROM orders
            WHERE transaction_id = $1
            "#,
            id
        )
        .fetch_one(&mut *tx)
        .await
        .map(|rec| DbOrder {
            id: rec.id,
            transaction_id: rec.transaction_id,
            order_number: rec.order_number.unwrap(),
            delivery_type: rec.delivery_type,
            delivery_details: rec.delivery_details,
            created_at: rec.created_at,
        })?;

        debug!("Found order with id: {}", order.id);

        // Get order items
        let items: Vec<DbOrderItem> = sqlx::query!(
            r#"
            SELECT id, order_id, name, category, price, created_at as "created_at: DateTime<Utc>"
            FROM order_items
            WHERE order_id = $1
            "#,
            order.id
        )
        .fetch_all(&mut *tx)
        .await?
        .into_iter()
        .map(|row| {
            debug!("Found item with id: {}, price: {}", row.id, row.price);
            DbOrderItem {
                id: row.id,
                order_id: row.order_id,
                name: row.name,
                category: row.category,
                price: row.price,
                created_at: row.created_at,
            }
        })
        .collect();

        debug!("Found {} items for order {}", items.len(), order.id);

        // Get customer data
        let customer: DbCustomerData = sqlx::query!(
            r#"
            SELECT id, order_id, name, email, created_at as "created_at: DateTime<Utc>"
            FROM customers
            WHERE order_id = $1
            "#,
            order.id
        )
        .fetch_one(&mut *tx)
        .await
        .map(|rec| DbCustomerData {
            id: rec.id,
            order_id: rec.order_id,
            name: rec.name,
            email: rec.email,
            created_at: rec.created_at,
        })?;

        debug!("Found customer with id: {}", customer.id);

        // Get billing data
        let billing: DbBillingData = sqlx::query!(
            r#"
            SELECT id, order_id, payment_type, payment_details, billing_address, created_at as "created_at: DateTime<Utc>"
            FROM billing_data
            WHERE order_id = $1
            "#,
            order.id
        )
        .fetch_one(&mut *tx)
        .await
        .map(|rec| DbBillingData {
            id: rec.id,
            order_id: rec.order_id,
            payment_type: rec.payment_type,
            payment_details: rec.payment_details,
            billing_address: rec.billing_address,
            created_at: rec.created_at,
        })?;

        debug!("Found billing with id: {}", billing.id);

        tx.commit().await?;

        Ok(Order {
            order,
            items,
            customer,
            billing,
        })
    }

}

impl EcomOrderStorage {
    async fn save_db_transaction(
        &self,
        tx: &mut PgConnection,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        debug!(
            "Inserting transaction record for "
        );
        match sqlx::query!(
            r#"
            INSERT INTO transactions (created_at)
            VALUES (NOW())
            RETURNING id
            "#,
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



    async fn save_order(
        &self,
        transaction_id: ModelId,
        order: &ImportOrder,
        tx: &mut PgConnection,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        debug!(
            "Inserting main order record for order_id: {}",
            order.order_number
        );
        match sqlx::query!(
            r#"
            INSERT INTO orders (
                transaction_id, order_number, delivery_type, delivery_details
            ) VALUES ($1, $2, $3, $4)
            RETURNING id
            "#,
            transaction_id,
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
        order_id: ModelId,
        customer: &ImportCustomerData,
        tx: &mut PgConnection,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        debug!("Inserting customer data for order_id: {}", order_id);
        match sqlx::query!(
            r#"
            INSERT INTO customers (
                order_id, name, email
            ) VALUES ($1, $2, $3)
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
                Ok(rec.id)
            }
            Err(e) => {
                error!("Failed to insert customer data: {}", e);
                return Err(e.into());
            }
        }
    }

    async fn save_order_items(
        &self,
        order_id: ModelId,
        items: &[ImportOrderItem],
        tx: &mut PgConnection,
    ) -> Result<Vec<ModelId>, Box<dyn Error + Send + Sync>> {
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
                ) VALUES ($1, $2, $3, $4)
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
                    item_ids.push(rec.id);
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
        order_id: ModelId,
        billing: &ImportBillingData,
        tx: &mut PgConnection,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        debug!("Inserting billing info for order_id: {}", order_id);

        match sqlx::query!(
            r#"
            INSERT INTO billing_data (
                order_id, payment_type, payment_details, billing_address
            ) VALUES ($1, $2, $3, $4)
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
                Ok(rec.id)
            }
            Err(e) => {
                error!("Failed to insert billing info: {}", e);
                Err(e.into())
            }
        }
    }

}
