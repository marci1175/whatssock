use std::sync::Arc;

use axum::extract::ws::Message;
use dashmap::DashMap;
use diesel::{PgConnection, r2d2::ConnectionManager};
use tokio::sync::broadcast::Receiver;

pub mod api;
pub mod models;
pub mod schema;

pub type PgPool = r2d2::Pool<ConnectionManager<PgConnection>>;

#[derive(Debug, Clone)]
pub struct ServerState {
    pub pg_pool: PgPool,
    pub currently_online_chatrooms: Arc<DashMap<i32, Receiver<Message>>>,
    pub connected_users_websockets: (),
}
