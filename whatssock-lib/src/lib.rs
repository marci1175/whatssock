pub mod client;
pub mod server;

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
