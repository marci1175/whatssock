// @generated automatically by Diesel CLI.

diesel::table! {
    chatrooms (id) {
        id -> Int4,
        chatroom_id -> Varchar,
        chatroom_name -> Varchar,
        chatroom_password -> Nullable<Varchar>,
        participants -> Array<Nullable<Int4>>,
        is_direct_message -> Bool,
        last_message_id -> Nullable<Int4>,
    }
}

diesel::table! {
    messages (id) {
        id -> Int4,
        owner_user_id -> Int4,
        replying_to_msg -> Nullable<Int4>,
        parent_chatroom_id -> Int4,
        raw_message -> Bytea,
        send_date -> Timestamp,
    }
}

diesel::table! {
    posts (id) {
        id -> Int4,
        title -> Varchar,
        body -> Text,
        published -> Bool,
    }
}

diesel::table! {
    user_session_auth (token_id) {
        token_id -> Int4,
        user_id -> Int4,
        session_token -> Bytea,
        encryption_key -> Bytea,
    }
}

diesel::table! {
    users (id) {
        id -> Int4,
        username -> Varchar,
        passw -> Varchar,
        email -> Varchar,
        chatrooms_joined -> Array<Nullable<Int4>>,
        created_at -> Date,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    chatrooms,
    messages,
    posts,
    user_session_auth,
    users,
);
