use axum::{
    extract::{WebSocketUpgrade, ws::WebSocket},
    response::Response,
};
use whatssock_lib::WebSocketChatroomMessages;

pub async fn handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_socket)
}

pub async fn handle_socket(mut socket: WebSocket) {
    while let Some(msg) = socket.recv().await {
        let msg_bytes = if let Ok(msg) = msg {
            // All of the messages we send over are in data format
            // They are serialized via rmp_serde
            // All messages will have the type [`WebSocketChatroomMessages`]
            let msg_bytes = msg.into_data();

            // We can safely unwrap here
            let ws_msg = rmp_serde::from_slice::<WebSocketChatroomMessages>(&msg_bytes).unwrap();

            // Handle the incoming message
            match ws_msg {
                WebSocketChatroomMessages::Message(message) => {
                    dbg!(message);
                },
            }

            msg_bytes
        } else {
            // client disconnected
            return;
        };

        if socket.send(axum::extract::ws::Message::Binary(msg_bytes)).await.is_err() {
            // client disconnected
            return;
        }
    }
}
