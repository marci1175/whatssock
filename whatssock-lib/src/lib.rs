pub mod client;
pub mod server;

use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

use crate::client::WebSocketChatroomMessageClient;

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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Hash, Eq)]
pub struct FetchChatroomResponse {
    pub chatroom_uid: i32,
    pub chatroom_id: String,
    pub chatroom_name: String,
    /// The reason it is an option is because this is what diesel hehe
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
pub enum WebSocketChatroomMessages {
    StringMessage(String),
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct UserLookup {
    pub username: String,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Copy)]
pub struct BulkChatroomMsgRequest {
    pub chatroom_id: i32,
    pub count: i32,
    pub offset_id: i32,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize, Copy)]
pub enum MessageFetchType {
    BulkFromChatroom(BulkChatroomMsgRequest),
    SingluarFromId(i32),
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct FetchMessagesResponse {
    pub messages: Vec<ChatroomMessageResponse>,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct ChatroomMessageResponse {
    pub id: i32,
    pub parent_chatroom_id: i32,
    pub owner_user_id: i32,
    pub send_date: NaiveDateTime,
    pub raw_message: Vec<u8>,
}
