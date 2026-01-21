use async_trait::async_trait;
use common::test_helpers::{
    create_test_connection, get_test_database_url, setup_test_environment,
    truncate_processing_tables,
};
use ecom_f2::model::*;
use mockall::mock;
use mongodb::bson::oid::ObjectId;
use processing::storage::mongo_common::MongoCommonStorage;
use processing::{executable_utils::import_transaction};
use processing::{importer::Importer, queue::QueueService};
use std::error::Error;
use std::sync::Arc;
use tokio::sync::OnceCell;

static SETUP: OnceCell<()> = OnceCell::const_new();

async fn ensure_setup() {
    SETUP
        .get_or_init(|| async {
            setup_test_environment()
                .await
                .expect("Failed to setup test environment");
        })
        .await;
}

// Direct mockall implementation - much cleaner than custom adapter!
mock! {
    QueueService {}

    #[async_trait]
    impl QueueService<ObjectId> for QueueService {
        async fn fetch_next(&self, number: i32) -> Result<Vec<(ObjectId, i64)>, Box<dyn Error + Send + Sync>>;
        async fn mark_processed(&self, tx_id: i64) -> Result<(), Box<dyn Error + Send + Sync>>;
        async fn enqueue(&self, ids: &[ObjectId]) -> Result<(), Box<dyn Error + Send + Sync>>;
        async fn is_enqueued(&self, ids: &[ObjectId]) -> Result<Vec<ObjectId>, Box<dyn Error + Send + Sync>>;
    }
}

const SAMPLE_ORDER_JSON: &str = r#"
{
  "billingIdentity": {
    "dateOfBirth": "1994-09-13",
    "emailAddress": {
      "email": "70ac1e50-5aac-4fb4-9f24-b34430baa5c8@gmx.de"
    },
    "firstName": "AKvdmT",
    "lastName": "noGmI",
    "paymentDetails": {
      "method": "invoice",
      "type": "activePayment"
    },
    "phoneNumbers": [
      {
        "lastUpdatedDate": "2013-05-19T14:16:33Z",
        "phoneNumber": "4542427449"
      },
      {
        "lastUpdatedDate": "2013-05-19T14:16:33Z",
        "phoneNumber": "723911723715"
      }
    ],
    "sourceId": "2b6c5eee-a024-4744-aabb-61511f4a7207",
    "address": {
      "city": "Horgenzell",
      "country": "DE",
      "houseNumber": "2",
      "lastUpdatedDate": "2022-03-26T08:22:38Z",
      "postalCode": "88263",
      "street": "Alte Poststr"
    }
  },
  "customerDomain": "ecommerce",
  "customerAccount": {
    "createdDate": "2010-11-28T14:52:53Z",
    "firstOrderDate": "2010-12-03T14:52:53Z",
    "lastPaymentDate": "2022-05-24T00:00:00Z",
    "lastUpdatedPasswordDate": "2022-06-12T01:56:38Z",
    "numberOfPastOrders": 38,
    "openBalance": "3",
    "sourceId": "696ebb4b-7fe1-404b-a090-a550b807c7c7",
    "type": "new"
  },
  "deviatingShipmentIdentity": {
    "firstName": "AKvdmT",
    "lastName": "noGmI",
    "phoneNumbers": [
      {
        "phoneNumber": "723911723715"
      }
    ],
    "sourceId": "ac1513c0-c1f2-4119-8b6e-9acd8f6c22d1",
    "address": {
      "city": "Horgenzell",
      "country": "DE",
      "houseNumber": "2",
      "postalCode": "88263",
      "street": "Alte Poststr"
    }
  },
  "id": "be01fc3a-b482-498b-99a4-c63797cd2fb2",
  "order": {
    "channel": "internet",
    "channelDetail": "6470df83-4a30-43e0-b9ed-1fad4758624c",
    "date": "2022-06-12T02:00:05Z",
    "orderItems": [
      {
        "category": "97",
        "characteristic": "27",
        "name": "Teppich",
        "pricePerItem": "424.99",
        "quantity": 1,
        "shipment": {
          "state": "invoiced",
          "type": "desiredDate"
        },
        "sourceId": "cec48ff299bb8193d9392d1444e230f0"
      },
      {
        "category": "72",
        "characteristic": "71",
        "name": "Gläser-Set",
        "pricePerItem": "31.99",
        "quantity": 1,
        "shipment": {
          "state": "invoiced",
          "type": "normal"
        },
        "sourceId": "4e3cc603464ab4e794b87abb22ae810d"
      },
      {
        "category": "60",
        "characteristic": "72",
        "name": "Gardinenstange",
        "pricePerItem": "29.99",
        "quantity": 1,
        "shipment": {
          "state": "invoiced",
          "type": "normal"
        },
        "sourceId": "cd88660db89214e3b771c9324522b3e0"
      },
      {
        "category": "60",
        "characteristic": "72",
        "name": "Gardinenstange",
        "pricePerItem": "29.99",
        "quantity": 1,
        "shipment": {
          "state": "invoiced",
          "type": "normal"
        },
        "sourceId": "376b3c8018ac5b26965f83a2c3232839"
      },
      {
        "category": "60",
        "characteristic": "72",
        "name": "Gardinenstange",
        "pricePerItem": "29.99",
        "quantity": 1,
        "shipment": {
          "state": "invoiced",
          "type": "normal"
        },
        "sourceId": "d478a39e7474a00fb04085fa6dca9c7c"
      }
    ],
    "sourceId": "d909b8bf-8096-4d18-a887-a4ff5395db78",
    "totalPrice": "546.95"
  },
  "sessionId": "159267275",
  "shipment": {
    "state": "invoiced",
    "type": "desiredDate"
  },
  "tenantName": "ZJ",
  "customIds": [
    "3887243263",
    "3887243259",
    "3887243262",
    "3887243260",
    "3887243261"
  ],
  "deviceData": {
    "exactId": "AAxc1Q3ZBj3NV__qnzeHrDJVfa2g",
    "smartId": "A-4MjWJnNj8wTZ3Wel5fasc0DFl4"
  },
  "eventDate": "2022-06-12T02:00:58Z"
}
"#;

#[tokio::test]
async fn test_import_endpoint() -> Result<(), Box<dyn Error + Send + Sync>> {
    ensure_setup().await;
    println!("test_import_endpoint 1");

    // Clean up any existing test data first
    let database_url = get_test_database_url();
    let db = create_test_connection().await?;
    truncate_processing_tables(&db).await?;

    let storage = Arc::new(MongoCommonStorage::new(&database_url, "frida").await?);

    // Create mockall-based queue service with basic expectations 
    let mut queue = MockQueueService::new();
    queue.expect_fetch_next().returning(|_| Ok(vec![])); // Empty queue for this test
    queue.expect_mark_processed().returning(|_| Ok(()));
    queue.expect_enqueue().returning(|_| Ok(()));

    let queue = Arc::new(queue);

    // Deserialize fixture JSON and use it to simulate a real payload import
    let test_order: EcomF2Order = serde_json::from_str(SAMPLE_ORDER_JSON)
        .expect("sample ecom-f2 order json must deserialize");

    let importer = Importer::<EcomF2Order>::new(storage, queue);
    let resp =
        import_transaction::<EcomF2Order>(axum::extract::State(importer), axum::Json(test_order))
            .await;

    println!("test_import_endpoint: {:?}", resp);
    assert!(resp.status().is_success(), "Import endpoint should succeed");

    Ok(())
}
