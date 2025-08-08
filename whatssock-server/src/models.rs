use chrono::NaiveDateTime;
/// These structs contain all the types which are available for insertion in the db.
/// lib.rs contains the types which are necessary for the REST API.
use diesel::{
    Selectable,
    prelude::{AsChangeset, Insertable, Queryable, QueryableByName},
};

#[derive(Debug, Clone, Selectable, QueryableByName, Queryable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserAccountEntry {
    pub id: i32,
    pub username: String,
    pub passw: String,
    pub email: String,
    pub chatrooms_joined: Vec<Option<i32>>,
    pub created_at: chrono::NaiveDate,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewUserAccount {
    pub username: String,
    pub passw: String,
    pub chatrooms_joined: Vec<Option<i32>>,
    pub email: String,
}

#[derive(Debug, Clone, Selectable, QueryableByName, Queryable)]
#[diesel(table_name = crate::schema::user_signin_tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserSessionEntry {
    pub token_id: i32,
    pub user_id: i32,
    pub session_token: Vec<u8>,
}

#[derive(Debug, Clone, Insertable, AsChangeset)]
#[diesel(table_name = crate::schema::user_signin_tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewUserSession {
    pub user_id: i32,
    pub session_token: Vec<u8>,
}

#[derive(Debug, Clone, Selectable, QueryableByName, Queryable)]
#[diesel(table_name = crate::schema::chatrooms)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct ChatroomEntry {
    pub id: i32,
    pub chatroom_id: String,
    pub chatroom_name: String,
    pub chatroom_password: Option<String>,
    pub participants: Vec<Option<i32>>,
    pub is_direct_message: bool,
    pub last_message_id: Option<i32>,
}

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = crate::schema::chatrooms)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UpdateLastMessage {
    pub last_message_id: i32,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::chatrooms)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewChatroom {
    pub chatroom_id: String,
    pub chatroom_name: String,
    pub chatroom_password: Option<String>,
    pub participants: Vec<i32>,
    pub is_direct_message: bool,
    pub last_message_id: Option<i32>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct NewMessage {
    pub parent_chatroom_id: i32,
    pub owner_user_id: i32,
    pub send_date: NaiveDateTime,
    pub replying_to_msg: Option<i32>,
    pub raw_message: Vec<u8>,
}

#[derive(Debug, Clone, Selectable, QueryableByName, Queryable)]
#[diesel(table_name = crate::schema::messages)]
#[diesel(check_for_backend(diesel::pg::Pg))]
#[repr(C)]
pub struct MessageEntry {
    pub id: i32,
    pub owner_user_id: i32,
    pub replying_to_msg: Option<i32>,
    pub parent_chatroom_id: i32,
    pub raw_message: Vec<u8>,
    pub send_date: NaiveDateTime,
}
