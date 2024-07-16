mod db;
mod log;
mod rest;
mod view;

use crate::db::init_db;
use anyhow::Result;
use axum::{Extension, Router};
use sqlx::SqlitePool;
use tracing::info;

use crate::log::trace_layer;

/// Build the overall web service router.
/// Constructing the router in a function makes it easy to re-use in unit tests.
fn router(connection_pool: SqlitePool) -> Router {
    Router::new()
        // Nest service allows you to attach another router to a URL base.
        // "/" inside the service will be "/books" to the outside world.
        .nest_service("/books", rest::books_service())
        // Add the web view
        .nest_service("/", view::view_service())
        // Add the connection pool as a "layer", available for dependency injection.
        .layer(Extension(connection_pool))
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env if available
    dotenv::dotenv().ok();

    // Setup tracing
    // let file_appender = tracing_appender::rolling::daily("log", "axum.log");
    // let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    //
    // tracing_subscriber::fmt()
    //     .json()
    //     .with_line_number(true)
    //     // .with_target(false)
    //     .with_writer(non_blocking)
    //     // Build the subscriber
    //     .init();

    tracing_subscriber::fmt().with_line_number(true).init();

    // Initialize the database and obtain a connection pool
    let connection_pool = init_db().await?;

    // Initialize the Axum routing service
    let handle_service = router(connection_pool);
    let app = trace_layer(handle_service);
    // let app = Router::new()
    //     .merge(handle_service)
    //     .route_layer(middleware::from_fn(print_request_response))
    //     ;

    // Define the address to listen on (everything)
    // let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3001").await?;

    info!("Starting server");

    // Start the server
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}
