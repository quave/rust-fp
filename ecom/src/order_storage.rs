use crate::processible::EcomOrder;
use crate::import_model::*;
use crate::storage_model::{order, order_item, customer, billing_data};
use async_trait::async_trait;
use processing::model::ModelId;
use processing::storage::{ImportableStorage, ProcessibleStorage, WebStorage};
use seaography::{Builder, register_entities};
use tracing::{debug, error, info};
use sea_orm::{Database, DatabaseConnection, EntityTrait, ActiveModelTrait, Set, NotSet, TransactionTrait, ColumnTrait, QueryFilter};
use std::{error::Error, marker::PhantomData};


/// SeaORM-based storage implementation for ecom orders
/// This demonstrates how to implement processing traits using SeaORM
/// while keeping the implementation in the domain module (ecom)
pub struct OrderStorage {
    pub db: DatabaseConnection,
    _phantom: PhantomData<EcomOrder>,
}

impl OrderStorage {
    pub async fn new(database_url: &str) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let db = Database::connect(database_url).await?;
        Ok(Self {
            db,
            _phantom: PhantomData,
        })
    }

    /// Create an order with all related entities using SeaORM transaction
    async fn create_order_with_relations(
        &self,
        import_order: &ImportOrder,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        
        // Start a database transaction
        let txn = self.db.begin().await?;

        // Create the order
        let order = order::ActiveModel {
            id: NotSet,
            transaction_id: NotSet,
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
    async fn get_order_with_relations(&self, order_id: ModelId) -> Result<Option<EcomOrder>, Box<dyn Error + Send + Sync>> {
        // Get the order for this transaction
        let order = order::Entity::find()
            .filter(order::Column::Id.eq(order_id))
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

        Ok(Some(EcomOrder {
            order,
            items: order_items,
            customer,
            billing,
        }))
    }
}

// Implement the processing traits using SeaORM operations
#[async_trait]
impl ImportableStorage<ImportOrder> for OrderStorage {
    async fn save(
        &self,
        order: &ImportOrder,
    ) -> Result<ModelId, Box<dyn Error + Send + Sync>> {
        info!("Saving order using SeaORM: {}", order.order_number);
        // Create order with all relations
        let order_id = self.create_order_with_relations(order).await?;

        debug!("Successfully saved order {}", order.order_number);
        Ok(order_id)
    }
}

#[async_trait]
impl ProcessibleStorage<EcomOrder> for OrderStorage {
    async fn get_processible(&self, id: ModelId) -> Result<EcomOrder, Box<dyn Error + Send + Sync>> {
        debug!("Getting processible order using SeaORM for id: {}", id);

        match self.get_order_with_relations(id).await? {
            Some(order) => {
                debug!("Successfully retrieved order for id: {}", id);
                Ok(order)
            }
            None => {
                error!("Order not found for id: {}", id);
                Err(format!("Order not found for id: {}", id).into())
            }
        }
    }

    async fn set_transaction_id(
        &self,
        processible_id: ModelId,
        transaction_id: ModelId,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        use sea_orm::{EntityTrait, Set, ActiveModelTrait};

        // Update the order's transaction_id field
        let mut order: order::ActiveModel = order::Entity::find_by_id(processible_id)
            .one(&self.db)
            .await?
            .ok_or_else(|| format!("Order not found for id: {}", processible_id))?
            .into();

        order.transaction_id = Set(Some(transaction_id));

        order.update(&self.db).await?;

        Ok(())
    }
}

#[async_trait]
impl WebStorage<EcomOrder> for OrderStorage {
    fn get_connection(&self) -> &DatabaseConnection {
        &self.db
    }

    fn register_seaography_entities(&self, mut builder: Builder) -> Builder {
        register_entities!(
            builder,
            [
                order,
                order_item,
                customer,
                billing_data
            ]
        );
        builder
    }

    async fn get_web_transaction(&self, id: ModelId) -> Result<EcomOrder, Box<dyn Error + Send + Sync>> {
        debug!("Getting single transaction using SeaORM for id: {}", id);
        self.get_processible(id).await
    }
} 