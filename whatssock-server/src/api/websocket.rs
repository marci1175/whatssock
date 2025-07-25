use axum::{
    extract::{State, WebSocketUpgrade, ws::WebSocket},
    response::Response,
};
use whatssock_lib::server::WebSocketChatroomMessageServer;

use crate::{ServerState, api::chatrooms::handle_incoming_chatroom_message};

pub async fn handler(state: State<ServerState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|socket| handle_socket(state, socket))
}

pub async fn handle_socket(state: State<ServerState>, mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
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

        if socket
            .send(axum::extract::ws::Message::Binary(msg_bytes.into()))
            .await
            .is_err()
        {
            // client disconnected
            return;
        }
    }
}
