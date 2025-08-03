use std::{
    net::SocketAddr,
    sync::Arc,
};

use axum::{
    extract::{
        ConnectInfo, State, WebSocketUpgrade,
        ws::{Message, WebSocket},
    },
    response::Response,
};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use tokio::{
    select, spawn,
    sync::{
        broadcast::{Sender, channel},
        mpsc,
    },
};
use tokio_util::sync::CancellationToken;
use whatssock_lib::{UserSession, server::WebSocketChatroomMessageServer};

use crate::{
    ServerState,
    api::{
        chatrooms::handle_incoming_chatroom_message,
        user_account_control::{lookup_joined_chatrooms, verify_user_session},
    },
};

pub async fn handler(
    state: State<ServerState>,
    ws: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> Response {
    ws.on_upgrade(move |socket| handle_socket(state, socket, addr))
}

pub async fn handle_socket(state: State<ServerState>, socket: WebSocket, remote_addr: SocketAddr) {
    let (mut sender, mut reader) = socket.split();

    let (client_thread_sender_handle, mut sender_receiver) = mpsc::channel::<Message>(255);

    // Read authenticative first message
    if let Some(Ok(auth_msg)) = reader.next().await {
        // Get a db connection from the pool
        if let Ok(mut pg_connection) = state.pg_pool.get() {
            let msg_bytes = auth_msg.into_data();

            // We can safely unwrap here
            let user_session = rmp_serde::from_slice::<UserSession>(&msg_bytes).unwrap();

            if let Err(err) = verify_user_session(&user_session, &mut pg_connection) {
                error!("Error encountered when trying to authenticate WebSocket: {err}");

                // Close handler
                return;
            };

            let currently_available_chatroom_handlers = state.currently_online_chatrooms.clone();
            let chatroom_subscriptions_handle = state.chatroom_subscriptions.clone();

            // Get which chatrooms the user is present in
            match lookup_joined_chatrooms(&mut pg_connection, user_session.user_id) {
                Ok(joined_chatrooms) => {
                    // Automaticly subscribe to the chatrooms which the user has joined
                    for joined_chatroom in joined_chatrooms {
                        // We can safely unwrap here, the option is just a weird trait of diesel
                        let chatroom_id = joined_chatroom.unwrap();

                        if currently_available_chatroom_handlers
                            .get(&chatroom_id)
                            .is_none()
                        {
                            // Create chatroom handler if it doesnt exist yet
                            create_chatroom_handler(
                                chatroom_subscriptions_handle.clone(),
                                currently_available_chatroom_handlers.clone(),
                                chatroom_id,
                            );
                        }

                        // Subscribe to chatroom
                        subscribe_to_channel_handler(
                            chatroom_id,
                            user_session.user_id,
                            client_thread_sender_handle.clone(),
                            chatroom_subscriptions_handle.clone(),
                        );
                    }
                }
                Err(err) => {
                    error!(
                        "An error occured when trying to fetch which chatrooms the user was present in: {err}"
                    )
                }
            }

            let curr_open_conn = state.currently_open_connections.clone();
            let curr_open_conn_clone = curr_open_conn.clone();

            // Dont allow multiple ws connections from the same address
            if !curr_open_conn.insert(remote_addr) {
                error!("Remote: {remote_addr} tried to open two or more connections.");
                curr_open_conn.remove(&remote_addr);
                return;
            };

            // Spawn client receiver thread
            spawn(async move {
                loop {
                    while let Some(msg) = reader.next().await {
                        if let Ok(msg) = msg {
                            // All of the messages we send over are in data format
                            // They are serialized via rmp_serde
                            // All messages will have the type [`WebSocketChatroomMessage`]
                            let msg_bytes = msg.into_data();

                            // We can safely unwrap here
                            let ws_msg =
                                rmp_serde::from_slice::<WebSocketChatroomMessageServer>(&msg_bytes)
                                    .unwrap();

                            // Handle the incoming message
                            let relayed_message = match handle_incoming_chatroom_message(
                                &state,
                                ws_msg.clone(),
                            )
                            .await
                            {
                                Ok(relayed_msg) => relayed_msg,
                                Err(err) => {
                                    error!(
                                        "Error: `{err}` occured when trying to process incoming message from: `{}`. Quitting handler thread...",
                                        ws_msg.message_owner_session.user_id
                                    );
                                    curr_open_conn.remove(&remote_addr);
                                    break;
                                }
                            };

                            // Relay the message
                            // Check if there is a chatroom handler for this message
                            let chatroom_handler_sender = if let Some(sender_handle) =
                                currently_available_chatroom_handlers.get(&ws_msg.sent_to)
                            {
                                sender_handle.1.clone()
                            } else {
                                create_chatroom_handler(
                                    chatroom_subscriptions_handle.clone(),
                                    currently_available_chatroom_handlers.clone(),
                                    ws_msg.sent_to,
                                )
                            };

                            subscribe_to_channel_handler(
                                ws_msg.sent_to,
                                ws_msg.message_owner_session.user_id,
                                client_thread_sender_handle.clone(),
                                chatroom_subscriptions_handle.clone(),
                            );

                            chatroom_handler_sender
                                .send(Message::Binary(
                                    rmp_serde::to_vec(&relayed_message).unwrap().into(),
                                ))
                                .unwrap();
                        } else {
                            // client disconnected
                            break;
                        };
                    }
                }
            });

            // Spawn client writer
            spawn(async move {
                loop {
                    select! {
                        Some(message) = sender_receiver.recv() => {
                            sender.send(message).await.unwrap();
                        }
                        else => {
                            curr_open_conn_clone.remove(&remote_addr);

                            break;
                        }
                    }
                }
            });
        }
    }
}

pub fn subscribe_to_channel_handler(
    chatroom_id: i32,
    user_id: i32,
    client_handle: tokio::sync::mpsc::Sender<axum::extract::ws::Message>,
    chatroom_subscriptions: Arc<
        DashMap<i32, DashMap<i32, tokio::sync::mpsc::Sender<axum::extract::ws::Message>>>,
    >,
) {
    match chatroom_subscriptions.get_mut(&chatroom_id) {
        Some(mut handler) => {
            let websocket_list = handler.value_mut();

            websocket_list.insert(user_id, client_handle);
        }
        None => {
            error!("Tried to subscribe to non-existent chatroom handler. id: {chatroom_id}");
        }
    }
}

pub fn create_chatroom_handler(
    chatroom_subscriptions: Arc<
        DashMap<i32, DashMap<i32, tokio::sync::mpsc::Sender<axum::extract::ws::Message>>>,
    >,
    available_chatrooms_handle: Arc<DashMap<i32, (CancellationToken, Sender<Message>)>>,
    this_chatroom_id: i32,
) -> Sender<Message> {
    let (sender, mut receiver) = channel(255);
    let cancellation_token = CancellationToken::new();

    let cancellation_token_clone = cancellation_token.clone();

    // Store this chatroom's sender so that it can be accessed later
    available_chatrooms_handle.insert(this_chatroom_id, (cancellation_token, sender.clone()));
    chatroom_subscriptions.insert(this_chatroom_id, DashMap::new());

    // Spawn chatroom handler
    spawn(async move {
        loop {
            select! {
                _ = cancellation_token_clone.cancelled() => {
                    break;
                }

                // If this returns an error it means there are no more clients left.
                // We can close the chatroom handler if thats the case
                Ok(recv_msg) = receiver.recv() => {
                    // We can safely unwrap here
                    let chatroom_subs = chatroom_subscriptions.get(&this_chatroom_id).unwrap();

                    let chatroom_is_empty = {
                        let subs = chatroom_subs.value();

                        subs.retain(|user_id, user_subscription| {
                            if let Err(err) = user_subscription.try_send(recv_msg.clone()) {
                                error!("Error occured when sending to client `{user_id}` handler: {err}");

                                false
                            }
                            else {
                                true
                            }
                        });

                        subs.is_empty()
                    };

                    // If there are no more users left in the chatroom delete it.
                    if chatroom_is_empty {
                        info!("Removing chatroom: {this_chatroom_id} as there are no participants left.");
                        chatroom_subscriptions.remove(&this_chatroom_id);
                    }
                }
                else => {
                    break;
                }
            }
        }
    });

    sender
}
