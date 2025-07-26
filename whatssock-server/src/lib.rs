use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use diesel::{PgConnection, r2d2::ConnectionManager};
use futures_util::stream::SplitSink;
use tokio::sync::broadcast::{Receiver, Sender};
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
}
