use std::sync::Arc;

use crate::HttpClient;
use anyhow::ensure;
use reqwest::Response;
use whatssock_lib::{
    client::{LoginRequest, RegisterRequest},
    ChatMessage, CreateChatroomRequest, FetchKnownChatrooms, FetchUnknownChatroom, UserSession,
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

    pub async fn request_logout(&self, user_sesion: Arc<UserSession>) -> anyhow::Result<Response> {
        let response = self
            .client
            .post(format!("{}/api/logout", self.base_url))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&*user_sesion)?)
            .send()
            .await?;

        let response_code = response.status().as_u16();

        ensure!(response_code == 200, "Response code: {response_code}");

        Ok(response)
    }

    pub async fn fetch_unknown_chatroom(
        &self,
        fetch_chatroom_request: FetchUnknownChatroom,
    ) -> anyhow::Result<Response> {
        let response = self
            .client
            .post(format!("{}/api/request_unknown_chatroom", self.base_url))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&fetch_chatroom_request)?)
            .send()
            .await?;

        let response_code = response.status().as_u16();

        ensure!(response_code == 200, "Response code: {response_code}");

        Ok(response)
    }

    pub async fn fetch_known_chatrooms(
        &self,
        fetch_chatroom_request: FetchKnownChatrooms,
    ) -> anyhow::Result<Response> {
        let response = self
            .client
            .post(format!("{}/api/request_known_chatroom", self.base_url))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&fetch_chatroom_request)?)
            .send()
            .await?;

        let response_code = response.status().as_u16();

        ensure!(response_code == 200, "Response code: {response_code}");

        Ok(response)
    }

    pub async fn create_new_chatroom(
        &self,
        create_chatroom_request: CreateChatroomRequest,
    ) -> anyhow::Result<Response> {
        let response = self
            .client
            .post(format!("{}/api/chatroom_new", self.base_url))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&create_chatroom_request)?)
            .send()
            .await?;

        let response_code = response.status().as_u16();

        ensure!(response_code == 200, "Response code: {response_code}");

        Ok(response)
    }

    pub async fn send_message(&self, chat_message: ChatMessage) -> anyhow::Result<Response> {
        let response = self
            .client
            .post(format!("{}/api/chatroom_send_message", self.base_url))
            .header("Content-Type", "application/json")
            .body(serde_json::to_string(&chat_message)?)
            .send()
            .await?;

        let response_code = response.status().as_u16();

        ensure!(response_code == 200, "Response code: {response_code}");

        Ok(response)
    }
}
