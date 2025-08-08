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
    pub chatroom_uid: i32,
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

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
#[repr(C)]
pub struct ChatroomMessageResponse {
    pub message_id: i32,
    /// The userid of the sender of this message.
    pub message_owner_id: i32,
    /// The message's id this message was replying to.
    pub replying_to_msg_id: Option<i32>,
    /// The ID of the chatroom this message has been sent to.
    pub sent_to: i32,
    /// The raw message itself, as stored in the db
    pub raw_message: Vec<u8>,
    /// This will be overwritten by the server.
    pub date_issued: NaiveDateTime,
}

impl Into<WebSocketChatroomMessageClient> for ChatroomMessageResponse {
    fn into(self) -> WebSocketChatroomMessageClient {
        WebSocketChatroomMessageClient {
            message_id: self.message_id,
            message_owner_id: self.message_owner_id,
            replying_to_msg_id: self.replying_to_msg_id,
            sent_to: self.sent_to,
            message: {
                rmp_serde::from_slice::<WebSocketChatroomMessages>(&self.raw_message).expect("Raw message bytes from `ChatroomMessageResponse` cannot be converted into a valid `WebSocketChatroomMessages` type.")
            },
            date_issued: self.date_issued,
        }
    }
}

use std::mem::{size_of, align_of};

/// Cast a `Vec<T>` into a `Vec<U>` without copying.
///
/// # Safety
/// - `T` and `U` must have **identical memory layout**.
/// - Usually both should be `#[repr(C)]` (or `#[repr(transparent)]`) with the same fields.
/// - Breaking this rule = **undefined behavior**.
pub unsafe fn vec_cast<T, U>(mut v: Vec<T>) -> Vec<U> {
    const {
        if size_of::<T>() != size_of::<U>() {
            panic!("Size mismatch between types");
        }
        if align_of::<T>() != align_of::<U>() {
            panic!("Alignment mismatch between types");
        }
    }
    
    let ptr = v.as_mut_ptr() as *mut U;
    let len = v.len();
    let cap = v.capacity();
    std::mem::forget(v);
    unsafe {
        Vec::from_raw_parts(ptr, len, cap)
    }
}