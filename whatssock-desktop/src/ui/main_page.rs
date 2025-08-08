use std::{
    collections::{HashMap, VecDeque},
    rc::Rc,
    sync::Arc,
};

use dioxus::{prelude::*, web::WebEventExt};
use dioxus_toast::{ToastInfo, ToastManager};
use futures_util::StreamExt;
use parking_lot::Mutex;
use tokio::{
    select,
    sync::mpsc::{Receiver, Sender},
};
use tokio_tungstenite::tungstenite::Message;
use web_sys::{
    js_sys::{eval, global},
    wasm_bindgen::convert::ReturnWasmAbi,
};
use whatssock_lib::{
    client::{UserInformation, WebSocketChatroomMessageClient},
    server::WebSocketChatroomMessageServer,
    BulkChatroomMsgRequest, ChatroomMessageResponse, FetchChatroomResponse,
    FetchKnownChatroomResponse, FetchMessagesResponse, MessageFetchType, UserLookup, UserSession,
    WebSocketChatroomMessages,
};

use crate::{ApplicationContext, AuthHttpClient, HttpClient, Route};

#[component]
pub fn MainPage() -> Element {
    let (user_session, user_information) = use_context::<(UserSession, UserInformation)>();

    let (websocket_sender, remote_receiver) = use_context::<(
        Sender<WebSocketChatroomMessageServer>,
        Arc<Mutex<Receiver<Message>>>,
    )>();

    let http_client = use_context::<Arc<Mutex<HttpClient>>>().lock().clone();

    let application_ctx = provide_root_context(ApplicationContext {
        authed_http_client: AuthHttpClient::new(http_client, user_session.clone()),
        websocket_client_out: websocket_sender,
        websocket_client_in: remote_receiver,
    });

    let mut toast: Signal<ToastManager> = use_context();

    let user_session = Arc::new(user_session);
    let client = application_ctx.authed_http_client;
    let client_clone = client.clone();
    let client_clone_add_chatroom = client.clone();

    let navigator = navigator();

    let mut available_chatrooms: Signal<Vec<FetchChatroomResponse>> = use_signal(Vec::new);
    let mut cached_chat_messages: Signal<HashMap<i32, VecDeque<WebSocketChatroomMessageClient>>> =
        use_signal(HashMap::new);

    let mut chatroom_id_buffer = use_signal(String::new);
    let mut new_chatroom_name_buffer = use_signal(String::new);
    let mut chatroom_passw_buffer = use_signal(String::new);
    let mut chatroom_message_buffer = use_signal(String::new);
    let mut selected_chatroom_node_idx = use_signal(|| 0);

    let mut chatroom_last_messages_cache: Signal<HashMap<i32, ChatroomMessageResponse>> =
        use_signal(HashMap::new);

    let users_cache: Signal<HashMap<i32, UserLookup>> = use_signal(HashMap::new);
    let mut users_cache_writer = users_cache;

    let chatrooms_joined = user_information.chatrooms_joined;
    let client_chatroom_requester = client.clone();

    let currently_selected_chatroom_node: Memo<Option<FetchChatroomResponse>> =
        use_memo(move || {
            available_chatrooms
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
                            let ws_msg = rmp_serde::from_slice::<WebSocketChatroomMessageClient>(&received_bytes.into_data()).unwrap();

                            if let Some(chatroom) = cached_chat_messages.write().get_mut(&ws_msg.sent_to) {
                                chatroom.push_back(ws_msg);
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

            // If we have no chatrooms joined dont even bother fetching them
            if chatrooms_joined.is_empty() {
                return;
            }

            let chatroom_ids: Vec<i32> = chatrooms_joined.iter().map(|id| id.unwrap()).collect();

            let response = client.fetch_known_chatrooms(chatroom_ids).await.unwrap();

            let verified_chatrooms =
                serde_json::from_str::<FetchKnownChatroomResponse>(&response.text().await.unwrap())
                    .unwrap();

            for chatroom in &verified_chatrooms.chatrooms {
                cached_chat_messages
                    .write()
                    .insert(chatroom.chatroom_uid, VecDeque::new());
            }

            available_chatrooms.extend(verified_chatrooms.chatrooms);
        });
    });

    let user_requester_client = client.clone();

    // Create a UserInformation requesting coroutine
    // If it receives a message, it fetches the infromation from the server and stores it in `users_cache_writer`
    let user_requester_sender = Arc::new(use_coroutine(
        move |mut receiver: UnboundedReceiver<i32>| {
            let user_requester_client = user_requester_client.clone();
            async move {
                loop {
                    select! {
                        Some(message_owner_id) = receiver.next() => {
                            let lookup_response = user_requester_client.fetch_user_information(message_owner_id).await.unwrap();

                            let lookup = serde_json::from_str::<UserLookup>(&lookup_response.text().await.unwrap()).unwrap();

                            users_cache_writer.write().insert(message_owner_id, lookup);
                        }
                        else => {
                            break;
                        }
                    }
                }
            }
        },
    ));

    let user_requester_client = client.clone();

    // Create a last message requesting coroutine
    // It receives the message id and stores it in the messages cache specifically for this (`chatroom_last_messages_cache`)
    let chatroom_message_requester_sender = Arc::new(use_coroutine(
        move |mut receiver: UnboundedReceiver<MessageFetchType>| {
            let http_client = user_requester_client.clone();

            async move {
                loop {
                    select! {
                        Some(message_ftch) = receiver.next() => {
                            let fetch_response = http_client.fetch_messages(message_ftch).await.unwrap();

                            let messages_fetched = serde_json::from_str::<FetchMessagesResponse>(&fetch_response.text().await.unwrap()).unwrap();

                            match message_ftch {
                                MessageFetchType::BulkFromChatroom(bulk_chatroom_msg_request) => {
                                    let mut cached_msgs = cached_chat_messages.write();

                                    // We can safely unwrap here afaik
                                    let msg_list = cached_msgs.get_mut(&bulk_chatroom_msg_request.chatroom_uid).unwrap();

                                    for incoming_msg in messages_fetched.messages {
                                        msg_list.push_front(incoming_msg.into());
                                    }
                                },
                                MessageFetchType::SingluarFromId(_) => {
                                    let msg = &messages_fetched.messages[0];
                                    chatroom_last_messages_cache.write().insert(dbg!(msg.message_id), msg.clone());
                                },
                            }
                        }
                        else => {
                            break;
                        }
                    }
                }
            }
        },
    ));

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

                    {
                        rsx!(
                            for (idx, chatroom_node) in available_chatrooms.read().iter().enumerate() {
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
                                            match chatroom_node.last_message_id {
                                                Some(message_id) => {
                                                    match get_or_request_message_from_id(chatroom_last_messages_cache, chatroom_message_requester_sender.clone(), message_id) {
                                                        Some(last_msg) => {
                                                            rsx!(
                                                                div {
                                                                    id: "chatroom_last_message",
                                                                    {
                                                                        let username = match get_or_request_user_information(users_cache, user_requester_sender.clone(), last_msg.message_owner_id) {
                                                                                Some(user_lookup) => {
                                                                                    user_lookup.username
                                                                                },
                                                                                None => {
                                                                                    String::from("Loading...")
                                                                                },
                                                                            };

                                                                        // We can safely unwrap here
                                                                        let message_type = rmp_serde::from_slice::<WebSocketChatroomMessages>(&last_msg.raw_message).unwrap();

                                                                        match message_type {
                                                                            WebSocketChatroomMessages::StringMessage(message) => {
                                                                                rsx!(
                                                                                    div {
                                                                                        id: "chatroom_last_message",

                                                                                        div {
                                                                                            id: {
                                                                                                if username.clone() == user_information.username.clone() {
                                                                                                    "chatroom_last_message_name_owned"
                                                                                                }
                                                                                                else {
                                                                                                    "chatroom_last_message_name"
                                                                                                }
                                                                                            },

                                                                                            {
                                                                                                if username.clone() == user_information.username.clone() {
                                                                                                    "Me"
                                                                                                }
                                                                                                else {
                                                                                                    &username
                                                                                                }
                                                                                            }
                                                                                        }

                                                                                        div {
                                                                                            id: "chatroom_last_message_body",

                                                                                            { message }
                                                                                        }
                                                                                    }
                                                                                )
                                                                            },
                                                                        }
                                                                    }
                                                                }
                                                            )
                                                        },
                                                        None => {
                                                            rsx!("Loading...")
                                                        },
                                                    }
                                                },
                                                None => {
                                                    rsx!(
                                                        div {
                                                            id: "chatroom_last_message",

                                                            "No messages yet."
                                                        }
                                                    )
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                        )
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

                                                cached_chat_messages.write().insert(serialized_response.chatroom_uid, VecDeque::new());
                                                available_chatrooms.write().push(serialized_response);
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

                                                cached_chat_messages.write().insert(added_chatroom.chatroom_uid, VecDeque::new());
                                                available_chatrooms.write().push(added_chatroom);
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
                    onscroll: move |_: Event<ScrollData>| {let chatroom_message_requester_sender = chatroom_message_requester_sender.clone(); async move {
                        // Get how much we have scrolled
                        let scroll_top = document::eval("return chats.scrollTop").await.unwrap().to_string().parse::<i32>().unwrap();

                        // Request more messages to display from the server if the scroll is 0
                        if scroll_top == 0 {
                            chatroom_message_requester_sender.send(MessageFetchType::BulkFromChatroom(BulkChatroomMsgRequest { chatroom_uid: currently_selected_chatroom_node.unwrap().chatroom_uid, count: 10, offset_id: cached_chat_messages.read().get(&currently_selected_chatroom_node.unwrap().chatroom_uid).unwrap().get(0).unwrap().message_id }));
                        }
                    }},

                    if let Some(currently_selected_chatroom_node) = currently_selected_chatroom_node.read().clone() {
                        {
                            let chatroom_msgs_read = cached_chat_messages.read();
                            let chatroom_msgs = chatroom_msgs_read.get(&currently_selected_chatroom_node.chatroom_uid).unwrap();

                            if chatroom_msgs.is_empty() {
                                chatroom_message_requester_sender.send(MessageFetchType::BulkFromChatroom(BulkChatroomMsgRequest { chatroom_uid: currently_selected_chatroom_node.chatroom_uid, count: 10, offset_id: 0 }));
                            }

                            rsx!(
                                for chatroom_msg in chatroom_msgs {
                                    div {
                                        id: "message_node",
                                        // Display who sent the message
                                        {
                                            rsx!(
                                                div {
                                                    id: "message_author",

                                                    title: {
                                                        format!("User ID: {}", chatroom_msg.message_owner_id)
                                                    },

                                                    {
                                                        let message_owner_id = chatroom_msg.message_owner_id;

                                                        match get_or_request_user_information(users_cache, user_requester_sender.clone(), message_owner_id) {
                                                            Some(user_information) => {
                                                                if message_owner_id == user_session.user_id {
                                                                    String::from("Me")
                                                                }
                                                                else {
                                                                    user_information.username.clone()
                                                                }
                                                            },
                                                            None => {
                                                                // Display loading title
                                                                String::from("loading...")
                                                            },
                                                        }
                                                    }
                                                }
                                            )
                                        }

                                        // Display the message itself
                                        {
                                            rsx!(
                                                div {
                                                    id: "message_content",
                                                    match &chatroom_msg.message {
                                                        WebSocketChatroomMessages::StringMessage(message) => {
                                                            rsx!(
                                                                div {
                                                                    id: "string_message",

                                                                    { message.to_string() }
                                                                }
                                                            )
                                                        }
                                                    }
                                                }
                                            )
                                        }

                                        // Display the date it was sent
                                        {
                                            rsx!(
                                                div {
                                                    id: "message_date",

                                                    { chatroom_msg.date_issued.to_string() }
                                                }
                                            )
                                        }
                                    }
                                }
                            )
                        }}
                }

                // Bottompanel
                // Hold the chat inputs such as emojis text, etc.
                div {
                    class: "bottompanel",

                    {
                        if let Some(chatroom_info) = currently_selected_chatroom_node.read().clone() {
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
                                            let user_session = (*user_session).clone();
                                            let chatroom_message_sender = chatroom_message_sender.clone();
                                            let message = chatroom_message_buffer.to_string();

                                            // Make it so that we cant send out empty messages
                                            if !message.trim().is_empty() {
                                                spawn(async move {
                                                    chatroom_message_sender.send(WebSocketChatroomMessageServer::new(user_session, None, chatroom_info.chatroom_uid, WebSocketChatroomMessages::StringMessage(message.to_string()),  chrono::Utc::now().naive_local())).await.unwrap();
                                                });
                                            }
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

pub fn get_or_request_user_information(
    user_info_cache: Signal<HashMap<i32, UserLookup>>,
    user_requester_sender: Arc<Coroutine<i32>>,
    user_id: i32,
) -> Option<UserLookup> {
    // Try getting the UserLookup info from the cache
    match user_info_cache.read().get(&user_id) {
        // if it is found return it
        Some(user_information) => Some(user_information.clone()),
        // if its not present send the user id to the lookup request thread so that it will request it and write it into the cache
        None => {
            user_requester_sender.send(user_id);

            None
        }
    }
}

pub fn get_or_request_message_from_id(
    last_message_cache: Signal<HashMap<i32, ChatroomMessageResponse>>,
    message_requester_sender: Arc<Coroutine<MessageFetchType>>,
    msg_id: i32,
) -> Option<ChatroomMessageResponse> {
    // Try getting the message from the cache
    match last_message_cache.read().get(&msg_id) {
        Some(last_message) => Some(last_message.clone()),
        None => {
            message_requester_sender.send(MessageFetchType::SingluarFromId(msg_id));

            None
        }
    }
}
