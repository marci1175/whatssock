use std::{env, net::SocketAddr, sync::Arc};

use axum::{
    Router,
    body::Body,
    extract::Request,
    http::{Response, StatusCode},
    middleware::{self, Next},
    routing::{any, get, post},
    serve,
};
use dashmap::{DashMap, DashSet};
use diesel::{
    PgConnection,
    r2d2::{self, ConnectionManager},
};
use dotenvy::dotenv;
use env_logger::Env;
use log::info;
use tokio::net::TcpListener;
use whatssock_lib::domain_paths::{GET_FETCH_MESSAGES, GET_FETCH_USER, POST_LOGIN, POST_LOGOUT, POST_NEW_CHATROOM, POST_REGISTER, POST_REQUEST_K_CHATROOM, POST_REQUEST_UK_CHATROOM, POST_SESSION_VERIFICATION, WS_ESTABLISH_CHATROOM_CONNECTION};
use whatssock_server::{
    ServerState,
    api::{
        chatrooms::{
            create_chatroom, fetch_known_chatrooms, fetch_messages, fetch_unknown_chatroom,
            fetch_user,
        },
        user_account_control::{
            fetch_login, fetch_user_information_from_session, handle_logout_request, register_user,
        },
        websocket::handler,
    },
};

async fn log_request(request: Request<Body>, next: Next) -> Result<Response<Body>, StatusCode> {
    let method = request.method().clone();
    let uri = request.uri().clone();

    println!("> Incoming: {} {}", method, uri);
    println!("> Headers: {:?}", request.headers());

    let response = next.run(request).await;

    println!("< Response status: {}", response.status());

    Ok(response)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    console_subscriber::init();

    info!("In italizing server state...");

    // Establish connection with the database
    let servere_state = establish_state()?;

    // Start up the webserver
    let router = Router::new()
        .route(POST_REGISTER, post(register_user))
        .route(POST_LOGIN, post(fetch_login))
        .route(POST_SESSION_VERIFICATION, post(fetch_user_information_from_session))
        .route(POST_LOGOUT, post(handle_logout_request))
        .route(
            POST_REQUEST_UK_CHATROOM,
            post(fetch_unknown_chatroom),
        )
        .route(POST_REQUEST_K_CHATROOM, post(fetch_known_chatrooms))
        .route(POST_NEW_CHATROOM, post(create_chatroom))
        .route(GET_FETCH_USER, get(fetch_user))
        .route(GET_FETCH_MESSAGES, get(fetch_messages))
        .route(WS_ESTABLISH_CHATROOM_CONNECTION, any(handler))
        .layer(middleware::from_fn(log_request))
        .with_state(servere_state);

    info!("Initalizing tcp listener...");

    let listener = TcpListener::bind("[::1]:3004").await?;

    info!("Serving service...");

    serve(
        listener,
        router.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

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
        currently_open_connections: Arc::new(DashSet::new()),
    })
}
