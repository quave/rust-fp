use chrono::NaiveDateTime;
use sea_orm::entity::prelude::*;

pub mod processing_queue {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "processing_queue")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub transaction_id: i64,
        pub processed_at: Option<NaiveDateTime>,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod recalculation_queue {
    use super::*;

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
    #[sea_orm(table_name = "recalculation_queue")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub transaction_id: i64,
        pub processed_at: Option<NaiveDateTime>,
        pub created_at: NaiveDateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
