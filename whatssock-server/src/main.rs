use std::{env, sync::Arc};

use axum::{
    Router,
    routing::{any, get, post},
    serve,
};
use dashmap::DashMap;
use diesel::{
    PgConnection,
    r2d2::{self, ConnectionManager},
};
use dotenvy::dotenv;
use env_logger::Env;
use log::{error, info};
use tokio::net::TcpListener;
use whatssock_server::{
    ServerState,
    api::{
        chatrooms::{create_chatroom, fetch_known_chatrooms, fetch_unknown_chatroom, fetch_user},
        user_account_control::{
            fetch_login, fetch_session_token, handle_logout_request, register_user,
        },
        websocket::handler,
    },
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    console_subscriber::init();

    info!("Initalizing server state...");
    
    // Establish connection with the database
    let servere_state = establish_state()?;

    // Start up the webserver
    let router = Router::new()
        .route("/api/register", post(register_user))
        .route("/api/login", post(fetch_login))
        .route("/api/session", post(fetch_session_token))
        .route("/api/logout", post(handle_logout_request))
        .route(
            "/api/request_unknown_chatroom",
            post(fetch_unknown_chatroom),
        )
        .route("/api/request_known_chatroom", post(fetch_known_chatrooms))
        .route("/api/chatroom_new", post(create_chatroom))
        .route("/api/fetch_user", get(fetch_user))
        .route("/ws/chatroom", any(handler))
        .with_state(servere_state);

    info!("Initalizing tcp listener...");
        
    let listener = TcpListener::bind("[::1]:3004").await?;

    info!("Serving service...");

    serve(listener, router).await?;

    Ok(())
}

/// Establishes connection with the PostgreSQL database.
pub fn establish_state() -> anyhow::Result<ServerState> {
    // Read the database url from the .env
    dotenv().ok();

    // Fetch the DATABASE URL
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    info!("Connecting to DB...");

    // Establish connection with the database
    let pg_pool: r2d2::Pool<ConnectionManager<PgConnection>> =
        r2d2::Builder::new().build(ConnectionManager::new(database_url))?;

    Ok(ServerState {
        pg_pool,
        chatroom_subscriptions: Arc::new(DashMap::new()),
        currently_online_chatrooms: Arc::new(DashMap::new()),
    })
}
