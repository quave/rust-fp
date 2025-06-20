use actix_web::{test, web, App};
use frida_core::{
    importer::Importer, in_memory_queue::InMemoryQueue, queue_service::QueueService,
    storage::Storage,
};
use frida_ecom::{
    ecom_db_model::*, ecom_import_model::*, import_transaction,
    sqlite_order_storage::SqliteOrderStorage,
};
use std::sync::Once;

static INIT: Once = Once::new();

fn init_test_logging() {
    INIT.call_once(|| {
        env_logger::builder()
            .filter_level(log::LevelFilter::Debug)
            .is_test(true)
            .init();
    });
}

#[actix_web::test]
async fn test_import_endpoint() {
    init_test_logging();
    // Initialize test dependencies
    let storage = SqliteOrderStorage::new(":memory:")
        .await
        .expect("Failed to create test storage");
    storage
        .initialize_schema()
        .await
        .expect("Failed to initialize schema");
    let queue: InMemoryQueue<Order> = InMemoryQueue::new();

    let service = web::Data::new(Importer::<
        Order,
        ImportOrder,
        SqliteOrderStorage,
        InMemoryQueue<Order>,
    >::new(storage, queue));

    // Create test app
    let app = test::init_service(App::new().app_data(service.clone()).route(
        "/import",
        web::post().to(import_transaction::<
            Order,
            ImportOrder,
            SqliteOrderStorage,
            InMemoryQueue<Order>,
        >),
    ))
    .await;

    // Test valid order
    let valid_order = ImportOrder {
        order_number: "ORD123".to_string(),
        customer: ImportCustomerData {
            name: "Test Customer".to_string(),
            email: "test@example.com".to_string(),
        },
        items: vec![ImportOrderItem {
            name: "Test Item".to_string(),
            category: "Test Category".to_string(),
            price: 10.99,
        }],
        billing: ImportBillingData {
            payment_type: "credit_card".to_string(),
            payment_details: "4242424242424242".to_string(),
            billing_address: "123 Test St".to_string(),
        },
        delivery_type: "standard".to_string(),
        delivery_details: "Test Address".to_string(),
    };

    let resp = test::TestRequest::post()
        .uri("/import")
        .set_json(&valid_order)
        .send_request(&app)
        .await;

    let status = resp.status();
    log::info!("Response: {:?}", resp);
    let body = test::read_body(resp).await;
    log::info!("Response body: {:?}", String::from_utf8_lossy(&body));

    assert_eq!(status, 200);

    // Test invalid order
    let invalid_order = serde_json::json!({
        "id": "",
    });

    let resp = test::TestRequest::post()
        .uri("/import")
        .set_json(&invalid_order)
        .send_request(&app)
        .await;

    assert_eq!(resp.status(), 400);
}
