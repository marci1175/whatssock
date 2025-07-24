use std::{
    fs,
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::{Arc, LazyLock},
};

use dioxus::prelude::Routable;
use dioxus::prelude::*;
use dirs::data_local_dir;
use parking_lot::Mutex;
use reqwest::Client;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio_tungstenite::tungstenite::Message;
use whatssock_lib::{UserSession, WebSocketChatroomMessage};
pub mod api_requests;
pub mod authentication;
pub mod ui;
use crate::ui::{login::Login, main_page::MainPage, not_found::NotFound, register::Register};

#[derive(Debug, Clone)]
pub struct HttpClient {
    pub client: Client,
    pub base_url: String,
}

impl HttpClient {
    pub fn new(client: Client, base_url: String) -> Self {
        Self { client, base_url }
    }
}

impl Deref for HttpClient {
    type Target = Client;

    fn deref(&self) -> &Self::Target {
        &self.client
    }
}

impl DerefMut for HttpClient {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.client
    }
}

#[derive(Debug, Clone)]
pub struct AuthHttpClient {
    client: HttpClient,
    user_session: UserSession,
}

impl AuthHttpClient {
    pub fn new(client: HttpClient, user_session: UserSession) -> Self {
        Self {
            client,
            user_session,
        }
    }
}

#[derive(Routable, PartialEq, Clone)]
pub enum Route {
    #[route("/")]
    Login {},
    #[route("/register")]
    Register {},
    #[route("/chats")]
    MainPage {},
    #[route("/:..segments")]
    NotFound { segments: Vec<String> },
}

pub static COOKIE_SAVE_PATH: LazyLock<PathBuf> = LazyLock::new(|| {
    let mut cookie_save_path = data_local_dir().unwrap();

    cookie_save_path.push("/.whatssock");

    if !fs::exists(&cookie_save_path).unwrap_or_default() {
        fs::create_dir_all(&cookie_save_path).unwrap_or_default();
    }

    cookie_save_path.push("user_session");

    cookie_save_path
});

pub type HttpWebClient = Arc<Mutex<HttpClient>>;

#[derive(Clone)]
pub struct ApplicationContext {
    pub authed_http_client: AuthHttpClient,
    pub websocket_client_out: Sender<WebSocketChatroomMessage>,
    pub websocket_client_in: Arc<Mutex<Receiver<Message>>>,
}
