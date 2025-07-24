use std::sync::Arc;

use dioxus::prelude::*;
use dioxus_toast::{ToastInfo, ToastManager};
use parking_lot::Mutex;
use tokio::{select, sync::mpsc::{Receiver, Sender}};
use tokio_tungstenite::tungstenite::Message;
use whatssock_lib::{
    client::UserInformation, FetchChatroomResponse,
    FetchKnownChatroomResponse, UserSession,
    WebSocketChatroomMessage, WebSocketChatroomMessages,
};

use crate::{
    ApplicationContext, AuthHttpClient,
    HttpClient, Route,
};

#[component]
pub fn MainPage() -> Element {
    let (websocket_sender, remote_receiver) = use_context::<(Sender<WebSocketChatroomMessage>, Arc<Mutex<Receiver<Message>>>)>();
    let (user_session, user_information) = use_context::<(UserSession, UserInformation)>();

    let http_client = use_context::<Arc<Mutex<HttpClient>>>().lock().clone();

    let application_ctx = use_root_context(|| ApplicationContext {
        authed_http_client: AuthHttpClient::new(
            http_client,
            user_session.clone(),
        ),
        websocket_client_out: websocket_sender,
        websocket_client_in: remote_receiver,
    });

    let mut toast: Signal<ToastManager> = use_context();

    let user_session = Arc::new(user_session);
    let user_session_clone = user_session.clone();

    let client = application_ctx.authed_http_client;
    let client_clone = client.clone();
    let client_clone_add_chatroom = client.clone();

    let navigator = navigator();
    let mut user_chat_entries: Signal<Vec<FetchChatroomResponse>> = use_signal(Vec::new);

    let mut chatroom_id_buffer = use_signal(String::new);
    let mut new_chatroom_name_buffer = use_signal(String::new);
    let mut chatroom_passw_buffer = use_signal(String::new);
    let mut chatroom_message_buffer = use_signal(String::new);
    let mut chatroom_messages: Signal<Vec<String>> = use_signal(Vec::new);
    let mut selected_chatroom_node_idx = use_signal(|| 0);

    let chatrooms_joined = user_information.chatrooms_joined;
    let client_chatroom_requester = client.clone();
    let currently_selected_chatroom_node: Memo<Option<FetchChatroomResponse>> =
        use_memo(move || {
            user_chat_entries
                .read()
                .get(*selected_chatroom_node_idx.read())
                .cloned()
        });

    let chatroom_message_sender = application_ctx.websocket_client_out;
    let websocket_receiver = application_ctx.websocket_client_in;

    use_hook(|| {
        spawn(async move {
            loop {
                let mut websocket = websocket_receiver.lock();

                select! {
                    recv = websocket.recv() => {
                        if let Some(received_bytes) = recv {
                            let ws_msg = rmp_serde::from_slice::<WebSocketChatroomMessage>(&received_bytes.into_data()).unwrap();

                            match ws_msg.message {
                                WebSocketChatroomMessages::Message(message) => {
                                    chatroom_messages.push(message);
                                },
                            }
                        }
                    }
                }
            }
        });
    });

    // Request all the chatrooms of the IDs which were included in the useraccount
    use_hook(|| {
        spawn(async move {
            let client = client_chatroom_requester.clone();
            let chatroom_ids: Vec<i32> = chatrooms_joined.iter().map(|id| id.unwrap()).collect();

            let response = client.fetch_known_chatrooms(chatroom_ids).await.unwrap();

            let verified_chatrooms =
                serde_json::from_str::<FetchKnownChatroomResponse>(&response.text().await.unwrap())
                    .unwrap();

            user_chat_entries.extend(verified_chatrooms.chatrooms);
        });
    });

    rsx! {
        div {
            class: "window",

            // Sidepanel
            // This holds the user management panel aswell as the menu to pick whichever chat you want to see and send messages in.
            div {
                class: "sidepanel",
                id: "sidepanel_left",

                div {
                    id: "sidepanel_title",
                    class: "title",
                    "Current Messages"
                }

                div {
                    id: "chatroom_node_list",

                    for (idx, chatroom_node) in user_chat_entries.read().iter().enumerate() {
                        button {
                            id: {
                                if idx == *selected_chatroom_node_idx.read() {
                                    "selected_chatroom_node"
                                }
                                else {
                                    "chatroom_node"
                                }
                            },
                            onclick: move |_| {
                                selected_chatroom_node_idx.set(idx);
                            },

                            div {
                                id: "chat_icon",
                                img {}
                            }

                            div {
                                id: "chatroom_node_title",

                                div {
                                    {
                                        chatroom_node.chatroom_name.clone()
                                    }
                                }
                            }

                            div {
                                id: "chatroom_node_last_message",

                                {
                                    format!("{:?}", chatroom_node.last_message_id)
                                }
                            }
                        }
                    }
                }

                div {
                    id: "user_control_panel_area",
                    {
                        format!("Logged in as: {}", user_information.username)
                    }

                    div {
                        id: "user_control_panel_buttons",
                        button {
                            id: "user_control_panel_button",
                            "Settings"
                        }

                        button {
                            id: "user_control_panel_button",
                            onclick: move |_event| {
                                let client = client.clone();
                                let user_session = user_session.clone();

                                spawn(async move {
                                    // Send the logout request
                                    client.request_logout().await.unwrap();

                                    // Reset root ctx for the session
                                    let mut session_ctx = use_context::<Signal<Option<(UserSession, UserInformation)>>>();
                                    session_ctx.set(None);

                                    navigator.replace(Route::Login {  });

                                    // Push the notification
                                    toast.write().popup(ToastInfo::simple("Successfully logged out!"));
                                });
                            },

                            "Logout"
                        }

                        div {
                            class: "dropdown",
                            button {
                                id: "user_control_panel_button",
                                "Add a new chat!"
                            },
                            div {
                                class: "dropdown_content",

                                div {
                                    id: "chat_id_input_row",

                                    button {
                                        id: "new_chat_button",
                                        class: "button",
                                        onclick: move |_| {
                                            let client = client_clone.clone();
                                            let user_session = user_session_clone.clone();

                                            spawn(async move {
                                                let response = client.fetch_unknown_chatroom(chatroom_id_buffer.to_string(), {
                                                    let passw_str = chatroom_passw_buffer.to_string();

                                                    if passw_str.is_empty() {
                                                        None
                                                    }
                                                    else {
                                                        Some(passw_str)
                                                    }
                                                }).await.unwrap();

                                                let serialized_response = serde_json::from_str::<FetchChatroomResponse>(&response.text().await.unwrap()).unwrap();

                                                user_chat_entries.push(serialized_response);
                                            });
                                        },

                                        "Add"
                                    }

                                    input {
                                        oninput: move |event| {
                                            chatroom_id_buffer.set(event.value());
                                        },
                                        placeholder: "Chat ID",
                                    }
                                    input {
                                        id: "chatroom_password_input",
                                        r#type: "password",
                                        oninput: move |event| {
                                            chatroom_passw_buffer.set(event.value());
                                        },
                                        placeholder: "Chat Password",
                                    }
                                }
                            }
                        }
                        div {
                            class: "dropdown",
                            button {
                                id: "user_control_panel_button",
                                "Create a new chatroom!"
                            }
                            div {
                                class: "dropdown_content",

                                div {
                                    id: "chat_id_input_row",
                                    button {
                                        class: "button",
                                        onclick: move |_| {
                                            let client = client_clone_add_chatroom.clone();

                                            spawn(async move {
                                                let response = client.create_new_chatroom(
                                                    new_chatroom_name_buffer.to_string(),
                                                    {
                                                        let entered_passw = chatroom_passw_buffer.to_string();

                                                        if entered_passw.is_empty() {
                                                            None
                                                        }
                                                        else {
                                                            Some(entered_passw)
                                                        }
                                                    }
                                                ).await.unwrap();

                                                let added_chatroom = serde_json::from_str::<FetchChatroomResponse>(&response.text().await.unwrap()).unwrap();

                                                user_chat_entries.push(added_chatroom);
                                            });
                                        },
                                        "Create"
                                    }
                                    input {
                                        id: "create_chatroom_name_input",
                                        oninput: move |event| {
                                            new_chatroom_name_buffer.set(event.value());
                                        },
                                        placeholder: "Chatroom name",
                                    }
                                    input {
                                        id: "chatroom_password_input",
                                        r#type: "password",
                                        oninput: move |event| {
                                            chatroom_passw_buffer.set(event.value());
                                        },
                                        placeholder: "Chatroom Password",
                                    }
                                }
                            }
                        }
                    }
        }
        }

            // Chatpanel
            // Displayes the messages in the currently selected chatroom. This also allows for interaction with the messages.
            div {
                class: "chatpanel",

                div {
                    id: "chats",

                    for chatroom_msg in chatroom_messages.read().iter() {
                        div {
                            {
                                chatroom_msg.to_string()
                            }
                        }
                    }
                }

                // Bottompanel
                // Hold the chat inputs such as emojis text, etc.
                div {
                    class: "bottompanel",

                    {
                        if let Some(chatroom_info) = (*currently_selected_chatroom_node.read()).clone() {
                            rsx! {
                                div {
                                    id: "chat_input_row",
                                    input {
                                        id: "chat_input",
                                        onchange: move |event| {
                                            chatroom_message_buffer.set(event.value());
                                        },
                                        placeholder: {
                                            format!("Message: {}", chatroom_info.chatroom_name)
                                        },
                                    }
                                    button {
                                        class: "button",
                                        id: "send_message_button",
                                        onclick: move |_| {
                                            let chatroom_message_sender = chatroom_message_sender.clone();
                                            spawn(async move {
                                                chatroom_message_sender.send(WebSocketChatroomMessage::new(user_information.user_id, None, chrono::Utc::now(), chatroom_info.chatroom_uid, WebSocketChatroomMessages::Message(chatroom_message_buffer.to_string()))).await.unwrap();
                                            });
                                        },

                                        "Send"
                                    }
                                }
                            }
                        }
                        else {
                            rsx!()
                        }
                    }

                }
            }
        }
    }
}
