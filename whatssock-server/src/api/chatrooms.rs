use crate::api::chatrooms::users::dsl::users;
use crate::api::user_account_control::verify_user_session;
use crate::schema::messages::dsl::messages;

use crate::models::{ChatroomEntry, MessageEntry, NewChatroom, NewMessage, UserAccountEntry};
use crate::schema::chatrooms::dsl::chatrooms;
use crate::schema::chatrooms::{chatroom_id, chatroom_password};
use crate::schema::user_signin_tokens::dsl::user_signin_tokens;
use crate::schema::users::{chatrooms_joined, id};
use crate::{
    ServerState,
    schema::{self, *},
};
use axum::{Json, extract::State, http::StatusCode};
use chrono::Utc;
use diesel::dsl::count_star;
use diesel::query_dsl::methods::{FilterDsl, SelectDsl};
use diesel::{ExpressionMethods, RunQueryDsl, SelectableHelper, insert_into};
use log::error;
use rand::Rng;
use rand::distr::Uniform;
use whatssock_lib::client::WebSocketChatroomMessageClient;
use whatssock_lib::server::WebSocketChatroomMessageServer;
use whatssock_lib::{
    CreateChatroomRequest, FetchChatroomResponse, FetchKnownChatroomResponse, FetchKnownChatrooms,
    FetchUnknownChatroom, UserLookup, UserSession,
};

pub async fn fetch_unknown_chatroom(
    State(state): State<ServerState>,
    Json(chatroom_request): Json<FetchUnknownChatroom>,
) -> Result<Json<FetchChatroomResponse>, StatusCode> {
    // Get a db connection from the pool
    let mut pg_connection = state.pg_pool.get().map_err(|err| {
        error!(
            "An error occured while fetching login information from db: {}",
            err
        );

        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let chatrooms_filter = chatrooms.filter(chatroom_id.eq(chatroom_request.chatroom_id));

    let query_result: ChatroomEntry = if let Some(password) = chatroom_request.password {
        let password_filter = chatrooms_filter.filter(chatroom_password.eq(password));

        password_filter
            .select(ChatroomEntry::as_select())
            .first(&mut pg_connection)
            .map_err(|err| {
                error!("An error occured while fetching chatrooms from db: {}", err);

                StatusCode::INTERNAL_SERVER_ERROR
            })?
    } else {
        chatrooms_filter
            .select(ChatroomEntry::as_select())
            .first(&mut pg_connection)
            .map_err(|err| {
                error!("An error occured while fetching chatrooms from db: {}", err);

                StatusCode::INTERNAL_SERVER_ERROR
            })?
    };

    Ok(Json(FetchChatroomResponse {
        chatroom_uid: query_result.id,
        chatroom_id: query_result.chatroom_id,
        chatroom_name: query_result.chatroom_name,
        participants: query_result.participants,
        is_direct_message: query_result.is_direct_message,
        last_message_id: query_result.last_message_id,
    }))
}

pub async fn fetch_known_chatrooms(
    State(state): State<ServerState>,
    Json(bulk_chatrooms_request): Json<FetchKnownChatrooms>,
) -> Result<Json<FetchKnownChatroomResponse>, StatusCode> {
    // Get a db connection from the pool
    let mut pg_connection = state.pg_pool.get().map_err(|err| {
        error!(
            "An error occured while fetching login information from db: {}",
            err
        );

        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Verify user session validness
    verify_user_session(&bulk_chatrooms_request.user_session, &mut pg_connection)?;

    let mut verified_chatrooms_reponses: Vec<FetchChatroomResponse> = Vec::new();

    // Verify that the user is indeed present in the chatroom
    for chatroom_request in bulk_chatrooms_request.chatroom_uids {
        let chatroom_entry = chatrooms
            .filter(schema::chatrooms::id.eq(chatroom_request))
            .get_result::<ChatroomEntry>(&mut pg_connection)
            .map_err(|err| {
                error!("An error occured while fetching chatrooms from db: {}", err);

                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        let is_user_present = chatroom_entry
            .participants
            .contains(&Some(bulk_chatrooms_request.user_session.user_id));

        // If the user is not present in the participants list, return an error
        if !is_user_present {
            return Err(StatusCode::FORBIDDEN);
        }

        verified_chatrooms_reponses.push(FetchChatroomResponse {
            chatroom_uid: chatroom_entry.id,
            chatroom_id: chatroom_entry.chatroom_id,
            chatroom_name: chatroom_entry.chatroom_name,
            participants: chatroom_entry.participants,
            is_direct_message: chatroom_entry.is_direct_message,
            last_message_id: chatroom_entry.last_message_id,
        });
    }

    Ok(Json(FetchKnownChatroomResponse {
        chatrooms: verified_chatrooms_reponses,
    }))
}

pub async fn create_chatroom(
    State(state): State<ServerState>,
    Json(chatroom_request): Json<CreateChatroomRequest>,
) -> Result<Json<FetchChatroomResponse>, StatusCode> {
    // Get a db connection from the pool
    let mut pg_connection = state.pg_pool.get().map_err(|err| {
        error!(
            "An error occured while fetching login information from db: {}",
            err
        );

        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let generated_chatroom_id: String = rand::rng()
        .sample_iter(&Uniform::new(char::from(32), char::from(126)).unwrap())
        .take(10)
        .collect();

    let chatroom_entry: ChatroomEntry = diesel::insert_into(chatrooms)
        .values(&NewChatroom {
            chatroom_id: generated_chatroom_id,
            chatroom_name: chatroom_request.chatroom_name,
            chatroom_password: chatroom_request.chatroom_passw,
            // Insert the user_id into the participants list
            participants: vec![chatroom_request.user_session.user_id],
            is_direct_message: false,
            last_message_id: None,
        })
        .get_result(&mut pg_connection)
        .map_err(|err| {
            error!("An error occured while creating a new chatroom: {}", err);

            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let mut user_account = users
        .filter(id.eq(chatroom_request.user_session.user_id))
        .get_result::<UserAccountEntry>(&mut pg_connection)
        .map_err(|err| {
            error!(
                "An error occured while fetching user account with id {}: {}",
                chatroom_request.user_session.user_id, err
            );

            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    user_account.chatrooms_joined.push(Some(chatroom_entry.id));

    diesel::update(users.filter(id.eq(chatroom_request.user_session.user_id)))
        .set(chatrooms_joined.eq(user_account.chatrooms_joined))
        .get_result::<UserAccountEntry>(&mut pg_connection)
        .map_err(|err| {
            error!(
                "An error occured while fetching user account with id {}: {}",
                chatroom_request.user_session.user_id, err
            );

            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(FetchChatroomResponse {
        chatroom_uid: chatroom_entry.id,
        chatroom_id: chatroom_entry.chatroom_id,
        chatroom_name: chatroom_entry.chatroom_name,
        participants: chatroom_entry.participants,
        is_direct_message: chatroom_entry.is_direct_message,
        last_message_id: chatroom_entry.last_message_id,
    }))
}

pub async fn handle_incoming_chatroom_message(
    State(state): &State<ServerState>,
    chatroom_request: WebSocketChatroomMessageServer,
) -> Result<WebSocketChatroomMessageClient, StatusCode> {
    // Get a db connection from the pool
    let mut pg_connection = state.pg_pool.get().map_err(|err| {
        error!(
            "An error occured while fetching login information from db: {}",
            err
        );

        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    verify_user_session(&chatroom_request.message_owner_session, &mut pg_connection)?;

    insert_into(messages)
        .values(NewMessage {
            parent_chatroom_id: chatroom_request.sent_to,
            owner_user_id: chatroom_request.message_owner_session.user_id,
            send_date: Utc::now().naive_utc(),
            raw_message: rmp_serde::to_vec(&chatroom_request.message).unwrap(),
        })
        .execute(&mut pg_connection)
        .map_err(|err| {
            error!("An error occured while processing message: {}", err);

            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(WebSocketChatroomMessageClient {
        message_owner_id: chatroom_request.message_owner_session.user_id,
        replying_to_msg_id: chatroom_request.replying_to_msg_id,
        sent_to: chatroom_request.sent_to,
        message: chatroom_request.message,
        date_issued: chatroom_request.date_issued,
    })
}

pub async fn fetch_user(
    State(state): State<ServerState>,
    uuid: String,
) -> Result<Json<UserLookup>, StatusCode> {
    let mut pg_connection = state.pg_pool.get().map_err(|err| {
        error!(
            "An error occured while fetching login information from db: {}",
            err
        );

        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let uuid = uuid.parse::<i32>().map_err(|_| { StatusCode::METHOD_NOT_ALLOWED })?;

    let query = users
        .filter(id.eq(uuid))
        .first::<UserAccountEntry>(&mut pg_connection)
        .map_err(|err| {
            error!(
                "An error occured while fetching user information from db: {}",
                err
            );

            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(UserLookup { username: query.username }))
}
