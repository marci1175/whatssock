use chrono::NaiveDateTime;

use crate::{UserSession, WebSocketChatroomMessages};

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LoginResponse {
    pub user_id: i32,
    pub session_token: [u8; 32],
    pub chatrooms_joined: Vec<Option<i32>>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LogoutResponse {}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct WebSocketChatroomMessageServer {
    /// The userid of the sender of this message.
    pub message_owner_session: UserSession,
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

impl WebSocketChatroomMessageServer {
    pub fn new(
        message_owner_session: UserSession,
        replying_to_msg_id: Option<i32>,
        sent_to: i32,
        message: WebSocketChatroomMessages,
        date_issued: NaiveDateTime,
    ) -> Self {
        Self {
            message_owner_session,
            replying_to_msg_id,
            sent_to,
            message,
            date_issued,
        }
    }
}
