use crate::model::{ModelId, WebTransaction};
use async_trait::async_trait;
use sea_orm::DatabaseConnection;
use seaography::{lazy_static::lazy_static, Builder, BuilderContext};
use std::error::Error;

lazy_static! { static ref CONTEXT : BuilderContext = BuilderContext :: default () ; }
const DEPTH_LIMIT: usize = 4;
const COMPLEXITY_LIMIT: usize = 100;

#[async_trait]
pub trait WebStorage<T: WebTransaction + Send + Sync>: Send + Sync {
    fn register_seaography_entities(&self, builder: Builder) -> Builder;

    fn get_connection(&self) -> &DatabaseConnection;

    fn get_seaography_schema(&self) -> async_graphql::dynamic::Schema {
        let database = self.get_connection();
        let builder = Builder::new(&CONTEXT, database.clone());
        
        let builder1 = self.register_seaography_entities(builder);

        // builder.register_enumeration::<crate::entities::sea_orm_active_enums::MpaaRating>();
        builder1
            .set_depth_limit(Some(DEPTH_LIMIT))
            .set_complexity_limit(Some(COMPLEXITY_LIMIT))
            .schema_builder()
            .data(database.clone())
            .finish()
            .expect("Failed to get SeaORM schema")
    }

    /// Get a specific transaction by ID
    async fn get_web_transaction(
        &self,
        transaction_id: ModelId,
    ) -> Result<T, Box<dyn Error + Send + Sync>>;


}