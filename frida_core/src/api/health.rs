#[get("/health")]
pub async fn health_check() -> HttpResponse {
    HttpResponse::Ok().finish()
} 