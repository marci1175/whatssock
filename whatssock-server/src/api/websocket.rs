use axum::{
    extract::{ws::WebSocket, State, WebSocketUpgrade},
    response::Response,
};
use whatssock_lib::{WebSocketChatroomMessage, WebSocketChatroomMessages};

use crate::ServerState;

pub async fn handler(state: State<ServerState>, ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|socket| handle_socket(state, socket))
}

pub async fn handle_socket(State(state): State<ServerState>, mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        let msg_bytes = if let Ok(msg) = msg {
            // All of the messages we send over are in data format
            // They are serialized via rmp_serde
            // All messages will have the type [`WebSocketChatroomMessage`]
            let msg_bytes = msg.into_data();

            // We can safely unwrap here
            let ws_msg = rmp_serde::from_slice::<WebSocketChatroomMessage>(&msg_bytes).unwrap();

            // Handle the incoming message
            match ws_msg.message {
                WebSocketChatroomMessages::Message(message) => {
                    dbg!(message);
                }
            }

            msg_bytes
        } else {
            // client disconnected
            return;
        };

        if socket
            .send(axum::extract::ws::Message::Binary(msg_bytes))
            .await
            .is_err()
        {
            // client disconnected
            return;
        }
    }
}
