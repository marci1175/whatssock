
use crate::{AuthHttpClient, HttpClient};
use anyhow::ensure;
use dioxus::logger::tracing::{error, info};
use futures_util::{SinkExt, StreamExt};
use reqwest::Response;
use tokio::{
    select,
    sync::mpsc::{channel, Receiver, Sender},
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use whatssock_lib::{
    client::{LoginRequest, RegisterRequest},
    ChatMessage, CreateChatroomRequest, FetchKnownChatrooms, FetchUnknownChatroom, UserSession,
    WebSocketChatroomMessage,
};

impl HttpClient {
    pub async fn fetch_login(
        &self,
        username: String,
        password: String,
    ) -> anyhow::Result<Response> {
        ensure!(!username.is_empty(), "Username must not be empty.");
        ensure!(!password.is_empty(), "Password must not be empty.");

        let response = self
            .client
            .post(format!("{}/api/login", self.base_url))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&LoginRequest { username, password })?)
            .send()
            .await?;

        let response_code = response.status().as_u16();

        ensure!(response_code == 200, "Response code: {response_code}");

        Ok(response)
    }

    pub async fn send_register_request(
        &self,
        username: String,
        password: String,
        email: String,
    ) -> anyhow::Result<Response> {
        ensure!(!username.is_empty(), "Username must not be empty.");
        ensure!(!password.is_empty(), "Password must not be empty.");
        ensure!(!email.is_empty(), "Email must not be empty.");

        let response = self
            .client
            .post(format!("{}/api/register", self.base_url))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&RegisterRequest {
                username,
                password,
                email,
            })?)
            .send()
            .await?;

        let response_code = response.status().as_u16();

        ensure!(response_code == 200, "Response code: {response_code}");

        Ok(response)
    }

    pub async fn verify_user_session(&self, user_sesion: UserSession) -> anyhow::Result<Response> {
        let response = self
            .client
            .post(format!("{}/api/session", self.base_url))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&user_sesion)?)
            .send()
            .await?;

        let response_code = response.status().as_u16();

        ensure!(response_code == 200, "Response code: {response_code}");

        Ok(response)
    }
}

impl AuthHttpClient {
    pub async fn request_logout(&self) -> anyhow::Result<Response> {
        let response = self
            .client
            .post(format!("{}/api/logout", self.client.base_url))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&self.user_session)?)
            .send()
            .await?;

        let response_code = response.status().as_u16();

        ensure!(response_code == 200, "Response code: {response_code}");

        Ok(response)
    }

    pub async fn fetch_unknown_chatroom(
        &self,
        chatroom_id: String,
        chatroom_passw: Option<String>,
    ) -> anyhow::Result<Response> {
        let response = self
            .client
            .post(format!(
                "{}/api/request_unknown_chatroom",
                self.client.base_url
            ))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&FetchUnknownChatroom {
                user_session: self.user_session.clone(),
                chatroom_id,
                password: chatroom_passw,
            })?)
            .send()
            .await?;

        let response_code = response.status().as_u16();

        ensure!(response_code == 200, "Response code: {response_code}");

        Ok(response)
    }

    pub async fn fetch_known_chatrooms(&self, chatroom_uids: Vec<i32>) -> anyhow::Result<Response> {
        let response = self
            .client
            .post(format!(
                "{}/api/request_known_chatroom",
                self.client.base_url
            ))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&FetchKnownChatrooms {
                user_session: self.user_session.clone(),
                chatroom_uids,
            })?)
            .send()
            .await?;

        let response_code = response.status().as_u16();

        ensure!(response_code == 200, "Response code: {response_code}");

        Ok(response)
    }

    pub async fn create_new_chatroom(
        &self,
        chatroom_name: String,
        chatroom_passw: Option<String>,
    ) -> anyhow::Result<Response> {
        let response = self
            .client
            .post(format!("{}/api/chatroom_new", self.client.base_url))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&CreateChatroomRequest {
                user_session: self.user_session.clone(),
                chatroom_name,
                chatroom_passw,
            })?)
            .send()
            .await?;

        let response_code = response.status().as_u16();

        ensure!(response_code == 200, "Response code: {response_code}");

        Ok(response)
    }

    pub async fn send_message(
        &self,
        destination_chatroom_id: i32,
        message: String,
    ) -> anyhow::Result<Response> {
        let response = self
            .client
            .post(format!(
                "{}/api/chatroom_send_message",
                self.client.base_url
            ))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&ChatMessage {
                user_session: self.user_session.clone(),
                destination_chatroom_id,
                message,
            })?)
            .send()
            .await?;

        let response_code = response.status().as_u16();

        ensure!(response_code == 200, "Response code: {response_code}");

        Ok(response)
    }
}

pub fn init_websocket_connection(
) -> (Sender<WebSocketChatroomMessage>, Receiver<Message>) {
    let (websocket_sender, mut websocket_receiver) = channel::<WebSocketChatroomMessage>(255);
    let (remote_sender, remote_receiver) = channel::<Message>(255);

    tokio::spawn(async move {
        let (ws_socket, response) = connect_async({
            #[cfg(debug_assertions)]
            {
                String::from("ws://[::1]:3004/ws/chatroom")
            }
            #[cfg(not(debug_assertions))]
            {
                String::from("ws://whatssock.com/ws/chatroom")
            }
        })
        .await
        .unwrap();

        info!("Successfully connected to the WebSocket.");

        let (mut write, mut read) = ws_socket.split();

        loop {
            select! {
                // This poll is going to wait until it receives a message from the client to send out a message.
                // It uses a mpsc to receive the messages from various points of the code.
                sendable_value = websocket_receiver.recv() => {
                    match sendable_value {
                        Some(message) => {
                            // Handle sending out the message through the websocket
                            write.send(Message::Binary(rmp_serde::to_vec(&message).unwrap().into())).await.unwrap();
                        },
                        None => {
                            error!("Websocket receiver handler channel closed. Websocket closed.");
                            break;
                        },
                    }
                },
                received_value = read.next() => {
                    if let Some(message) = received_value {
                        match message {
                            Ok(message) => {
                                remote_sender.send(message).await.unwrap();
                            },
                            Err(err) => {
                                error!("Error occured while reading a message from the WebSocket: {err}");
                            },
                        }
                    }
                }
            }
        }
    });

    (websocket_sender, remote_receiver)
}
