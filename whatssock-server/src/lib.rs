use std::{net::SocketAddr, sync::Arc};

use axum::extract::ws::Message;
use dashmap::{DashMap, DashSet};
use diesel::{PgConnection, r2d2::ConnectionManager};
use tokio::sync::{RwLock, broadcast::Sender};
use tokio_util::sync::CancellationToken;

pub mod api;
pub mod models;
pub mod schema;

pub type PgPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[derive(Debug, Clone)]
pub struct ServerState {
    pub pg_pool: PgPool,
    pub currently_online_chatrooms: Arc<
        DashMap<
            // Chatroom IDs
            i32,
            // Broadcast channel which is received as the chatroom handler thread
            (CancellationToken, Sender<Message>),
        >,
    >,
    pub chatroom_subscriptions:
        Arc<DashMap<i32, DashMap<i32, tokio::sync::mpsc::Sender<axum::extract::ws::Message>>>>,
    pub currently_open_connections: Arc<DashSet<SocketAddr>>,
}
