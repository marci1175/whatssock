pub mod client;
pub mod server;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Default, Debug, Serialize, Deserialize, PartialEq)]
pub struct UserSession {
    pub user_id: i32,
    pub session_token: [u8; 32],
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct FetchUnknownChatroom {
    pub user_session: UserSession,
    pub chatroom_id: String,
    pub password: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct FetchKnownChatrooms {
    pub user_session: UserSession,
    pub chatroom_uids: Vec<i32>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct FetchKnownChatroomResponse {
    pub chatrooms: Vec<FetchChatroomResponse>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct FetchChatroomResponse {
    pub chatroom_uid: i32,
    pub chatroom_id: String,
    pub chatroom_name: String,
    /// The reason it is an option is because this is what diesel returns
    pub participants: Vec<Option<i32>>,
    pub is_direct_message: bool,
    pub last_message_id: Option<i32>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct CreateChatroomRequest {
    pub user_session: UserSession,
    pub chatroom_name: String,
    pub chatroom_passw: Option<String>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct ChatMessage {
    pub user_session: UserSession,
    pub destination_chatroom_id: i32,
    pub message: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub enum WebSocketChatroomMessages {
    Message(String),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct MessageOwner {}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct WebSocketChatroomMessage {
    // The userid of the sender of this message.
    pub message_owner: i32,
    // The message's id this message was replying to.
    pub replying_to_msg_id: Option<i32>,
    // The date this message was sent at.
    pub sent_at: DateTime<Utc>,
    // The ID of the chatroom this message has been sent to.
    pub sent_to: i32,
    // The message itself.
    pub message: WebSocketChatroomMessages,
}

impl WebSocketChatroomMessage {
    pub fn new(
        message_owner: i32,
        replying_to_msg_id: Option<i32>,
        sent_at: DateTime<Utc>,
        sent_to: i32,
        message: WebSocketChatroomMessages,
    ) -> Self {
        Self {
            message_owner,
            replying_to_msg_id,
            sent_at,
            sent_to,
            message,
        }
    }
}
