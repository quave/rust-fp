use crate::models::Order;
use crate::ecom_import_model::*;
use crate::entities::{self as entities, order, order_item, customer, billing_data};
use async_trait::async_trait;
use processing::model::ModelId;
use processing::storage::{ImportableStorage, ProcessibleStorage, WebStorage};
use processing::ui_model::FilterRequest;
use tracing::{debug, error, info};
use sea_orm::{Database, DatabaseConnection, EntityTrait, ActiveModelTrait, Set, NotSet, TransactionTrait, ColumnTrait, QueryFilter};
use std::{error::Error, marker::PhantomData};


/// SeaORM-based storage implementation for ecom orders
/// This demonstrates how to implement processing traits using SeaORM
/// while keeping the implementation in the domain module (ecom)
pub struct EcomOrderStorage {
    pub db: DatabaseConnection,
    _phantom: PhantomData<Order>,
}

impl EcomOrderStorage {
    pub async fn new(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let db = Database::connect(database_url).await?;
        Ok(Self {
            db,
            _phantom: PhantomData,
        })
    }

    /// Create a transaction record using SeaORM
    async fn create_transaction(&self) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        let transaction = entities::ActiveModel {
            id: NotSet,
            label_id: Set(None),
            comment: Set(None),
            last_scoring_date: Set(None),
            processing_complete: Set(false),
            created_at: Set(chrono::Utc::now().naive_utc()),
        };

        let result = transaction.insert(&self.db).await?;
        Ok(result.id)
    }

    /// Create an order with all related entities using SeaORM transaction
    async fn create_order_with_relations(
        &self,
        transaction_id: ModelId,
        import_order: &ImportOrder,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        
        // Start a database transaction
        let txn = self.db.begin().await?;

        // Create the order
        let order = order::ActiveModel {
            id: NotSet,
            transaction_id: Set(transaction_id),
            order_number: Set(import_order.order_number.clone()),
            delivery_type: Set(import_order.delivery_type.clone()),
            delivery_details: Set(import_order.delivery_details.clone()),
            created_at: Set(chrono::Utc::now().naive_utc()),
        };

        let order_result = order.insert(&txn).await?;
        let order_id = order_result.id;

        // Create order items
        for item in &import_order.items {
            let order_item = order_item::ActiveModel {
                id: NotSet,
                order_id: Set(order_id),
                name: Set(item.name.clone()),
                category: Set(item.category.clone()),
                price: Set(item.price),
                created_at: Set(chrono::Utc::now().naive_utc()),
            };
            order_item.insert(&txn).await?;
        }

        // Create customer
        let customer = customer::ActiveModel {
            id: NotSet,
            order_id: Set(order_id),
            name: Set(import_order.customer.name.clone()),
            email: Set(import_order.customer.email.clone()),
            created_at: Set(chrono::Utc::now().naive_utc()),
        };
        customer.insert(&txn).await?;

        // Create billing data
        let billing = billing_data::ActiveModel {
            id: NotSet,
            order_id: Set(order_id),
            payment_type: Set(import_order.billing.payment_type.clone()),
            payment_details: Set(import_order.billing.payment_details.clone()),
            billing_address: Set(import_order.billing.billing_address.clone()),
            created_at: Set(chrono::Utc::now().naive_utc()),
        };
        billing.insert(&txn).await?;

        // Commit the transaction
        txn.commit().await?;

        Ok(order_id)
    }

    /// Retrieve an order with all related data using SeaORM
    async fn get_order_with_relations(&self, transaction_id: ModelId) -> Result<Option<Order>, Box<dyn Error + Send + Sync>> {
        // Find the transaction
        let transaction = entities::Entity::find_by_id(transaction_id)
            .one(&self.db)
            .await?;

        let Some(_transaction) = transaction else {
            return Ok(None);
        };

        // Get the order for this transaction
        let order = order::Entity::find()
            .filter(order::Column::TransactionId.eq(transaction_id))
            .one(&self.db)
            .await?;

        let Some(order) = order else {
            return Ok(None);
        };

        // Get related order items
        let order_items = order_item::Entity::find()
            .filter(order_item::Column::OrderId.eq(order.id))
            .all(&self.db)
            .await?;

        // Get related customers
        let customers = customer::Entity::find()
            .filter(customer::Column::OrderId.eq(order.id))
            .all(&self.db)
            .await?;

        // Get related billing data
        let billing_data = billing_data::Entity::find()
            .filter(billing_data::Column::OrderId.eq(order.id))
            .all(&self.db)
            .await?;

        // Use SeaORM entities directly in the logical Order model
        let customer = customers.into_iter().next()
            .ok_or("No customer data found for order")?;
        let billing = billing_data.into_iter().next()
            .ok_or("No billing data found for order")?;

        Ok(Some(Order {
            order,
            items: order_items,
            customer,
            billing,
        }))
    }
}

// Implement the processing traits using SeaORM operations
#[async_trait]
impl ImportableStorage<ImportOrder> for EcomOrderStorage {
    async fn save_transaction(
        &self,
        order: &ImportOrder,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        info!("Saving order using SeaORM: {}", order.order_number);

        // Create transaction record
        let transaction_id = self.create_transaction().await?;

        // Create order with all relations
        let _order_id = self.create_order_with_relations(transaction_id, order).await?;

        debug!("Successfully saved order {} with transaction_id {}", order.order_number, transaction_id);
        Ok(transaction_id)
    }
}

#[async_trait]
impl ProcessibleStorage<Order> for EcomOrderStorage {
    async fn get_processible(&self, id: ModelId) -> Result<Order, Box<dyn Error + Send + Sync>> {
        debug!("Getting processible order using SeaORM for transaction_id: {}", id);

        match self.get_order_with_relations(id).await? {
            Some(order) => {
                debug!("Successfully retrieved order for transaction_id: {}", id);
                Ok(order)
            }
            None => {
                error!("Order not found for transaction_id: {}", id);
                Err(format!("Order not found for transaction_id: {}", id).into())
            }
        }
    }
}

#[async_trait]
impl WebStorage<Order> for EcomOrderStorage {
    async fn get_transactions(&self, _filter: FilterRequest) -> Result<Vec<Order>, Box<dyn Error + Send + Sync>> {
        // For now, return empty - full filter implementation would require more complex SeaORM queries
        // This demonstrates the trait implementation pattern
        info!("Getting transactions using SeaORM (basic implementation)");
        Ok(Vec::new())
    }

    async fn get_transaction(&self, id: ModelId) -> Result<Order, Box<dyn Error + Send + Sync>> {
        debug!("Getting single transaction using SeaORM for id: {}", id);
        self.get_processible(id).await
    }
} 