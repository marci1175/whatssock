use std::sync::Arc;

use dioxus::prelude::*;
use dioxus_toast::{ToastInfo, ToastManager};
use whatssock_lib::{
    client::UserInformation, ChatMessage, CreateChatroomRequest, FetchChatroomResponse,
    FetchKnownChatroomResponse, FetchKnownChatrooms, FetchUnknownChatroom, UserSession, WebSocketChatroomMessages,
};

use crate::{ApplicationContext, Route};

#[component]
pub fn MainPage() -> Element {
    let application_ctx = use_context::<ApplicationContext>();

    let (user_session, user_information) = use_context::<(UserSession, UserInformation)>();
    let mut toast: Signal<ToastManager> = use_context();

    let user_session = Arc::new(user_session);
    let user_session_clone = user_session.clone();
    let user_session_clone_create_chatroom = user_session.clone();
    let user_session_clone_send_message = user_session.clone();

    let client = application_ctx.http_client;
    let client_clone = client.clone();
    let client_clone_add_chatroom = client.clone();
    let client_clone_send_message = client.clone();

    let navigator = navigator();
    let mut user_chat_entries: Signal<Vec<FetchChatroomResponse>> = use_signal(Vec::new);

    let mut chatroom_id_buffer = use_signal(String::new);
    let mut new_chatroom_name_buffer = use_signal(String::new);
    let mut chatroom_passw_buffer = use_signal(String::new);
    let mut chatroom_message_buffer = use_signal(String::new);

    let mut selected_chatroom_node_idx = use_signal(|| 0);

    let chatrooms_joined = user_information.chatrooms_joined;
    let client_chatroom_requester = client.clone();
    let user_session_chatroom_req = user_session.clone();
    let currently_selected_chatroom_node: Memo<Option<FetchChatroomResponse>> =
        use_memo(move || {
            user_chat_entries
                .read()
                .get(*selected_chatroom_node_idx.read())
                .cloned()
        });

    let chatroom_message_sender = application_ctx.websocket_client_out;

    // Request all the chatrooms of the IDs which were included in the useraccount
    use_hook(|| {
        spawn(async move {
            let client = client_chatroom_requester.lock();
            let chatroom_ids: Vec<i32> = chatrooms_joined.iter().map(|id| id.unwrap()).collect();

            let response = client
                .fetch_known_chatrooms(FetchKnownChatrooms {
                    user_session: (*user_session_chatroom_req).clone(),
                    chatroom_uids: chatroom_ids,
                })
                .await
                .unwrap();

            let verified_chatrooms =
                serde_json::from_str::<FetchKnownChatroomResponse>(&response.text().await.unwrap())
                    .unwrap();

            user_chat_entries.extend(dbg!(verified_chatrooms.chatrooms));
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
                                    client.lock().request_logout(user_session.clone()).await.unwrap();

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
                                                let response = client.lock().fetch_unknown_chatroom(FetchUnknownChatroom { user_session: (*user_session).clone(), chatroom_id: chatroom_id_buffer.to_string(), password: {
                                                    let passw_str = chatroom_passw_buffer.to_string();

                                                    if passw_str.is_empty() {
                                                        None
                                                    }
                                                    else {
                                                        Some(passw_str)
                                                    }
                                                } }).await.unwrap();

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
                                        onclick: move |event| {
                                            let client = client_clone_add_chatroom.clone();
                                            let user_session_clone_create_chatroom = user_session_clone_create_chatroom.clone();

                                            spawn(async move {
                                                let response = client.lock().create_new_chatroom(CreateChatroomRequest { chatroom_name: new_chatroom_name_buffer.to_string(), chatroom_passw: {
                                                    let entered_passw = chatroom_passw_buffer.to_string();
                                                    if entered_passw.is_empty() {
                                                        None
                                                    }
                                                    else {
                                                        Some(entered_passw)
                                                    }
                                                }, user_session: (*user_session_clone_create_chatroom).clone() }).await.unwrap();

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
                                            let client = client_clone_send_message.clone();
                                            let user_session = user_session_clone_send_message.clone();
                                            let destination_id = chatroom_info.chatroom_uid;
                                            let msg_buffer = chatroom_message_buffer;
                                            let chatroom_message_sender = chatroom_message_sender.clone();
                                            spawn(async move {
                                                let client = client.lock();
                                                
                                                chatroom_message_sender.send(WebSocketChatroomMessages::Message(msg_buffer.to_string())).await.unwrap();
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
