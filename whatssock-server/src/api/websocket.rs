use axum::{
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use log::error;
use whatssock_lib::{server::WebSocketChatroomMessageServer, UserSession};

use crate::{api::{chatrooms::handle_incoming_chatroom_message, user_account_control::verify_user_session}, ServerState};

pub async fn handler(state: State<ServerState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|socket| handle_socket(state, socket))
}

pub async fn handle_socket(state: State<ServerState>, socket: WebSocket) {
    let (mut sender, mut reader) = socket.split();

    // Read authenticative first message
    if let Some(Ok(auth_msg)) = reader.next().await {
        // Get a db connection from the pool
        if let Ok(mut pg_connection) = state.pg_pool.get() {
            let msg_bytes = auth_msg.into_data();

            // We can safely unwrap here
            let user_session =
                rmp_serde::from_slice::<UserSession>(&msg_bytes).unwrap();

            if let Err(err) = verify_user_session(&user_session, &mut pg_connection) {
                error!("Error encountered when trying to authenticate WebSocket: {err}");
                
                // Close handler
                return;
            };
        }
    }

    while let Some(msg) = reader.next().await {
        let msg_bytes = if let Ok(msg) = msg {
            // All of the messages we send over are in data format
            // They are serialized via rmp_serde
            // All messages will have the type [`WebSocketChatroomMessage`]
            let msg_bytes = msg.into_data();

            // We can safely unwrap here
            let ws_msg =
                rmp_serde::from_slice::<WebSocketChatroomMessageServer>(&msg_bytes).unwrap();

            // Handle the incoming message
            let relayed_message = handle_incoming_chatroom_message(&state, ws_msg)
                .await
                .unwrap();

            rmp_serde::to_vec(&relayed_message).unwrap()
        } else {
            // client disconnected
            return;
        };

        if sender
            .send(axum::extract::ws::Message::Binary(msg_bytes.into()))
            .await
            .is_err()
        {
            // client disconnected
            return;
        }
    }
}
