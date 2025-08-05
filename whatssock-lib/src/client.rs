use chrono::NaiveDateTime;

use crate::{MessageFetchType, UserSession, WebSocketChatroomMessages};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub email: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct UserInformation {
    pub username: String,
    pub chatrooms_joined: Vec<Option<i32>>,
    pub user_id: i32,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct WebSocketChatroomMessageClient {
    /// The userid of the sender of this message.
    pub message_owner_id: i32,
    /// The message's id this message was replying to.
    pub replying_to_msg_id: Option<i32>,
    /// The ID of the chatroom this message has been sent to.
    pub sent_to: i32,
    /// The message itself.
    pub message: WebSocketChatroomMessages,
    /// The date when it was sent.
    /// This will be overwritten by the server.
    pub date_issued: NaiveDateTime,
}

impl WebSocketChatroomMessageClient {
    pub fn new(
        message_owner_id: i32,
        replying_to_msg_id: Option<i32>,
        sent_to: i32,
        message: WebSocketChatroomMessages,
        date_issued: NaiveDateTime,
    ) -> Self {
        Self {
            message_owner_id,
            replying_to_msg_id,
            sent_to,
            message,
            date_issued,
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FetchMessages {
    pub user_session: UserSession,
    pub message_request: MessageFetchType,
}
