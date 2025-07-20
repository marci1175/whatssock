use dioxus::{logger::tracing::error, prelude::*};
use dioxus_toast::{ToastFrame, ToastManager};
use futures_util::{SinkExt, StreamExt};
use parking_lot::Mutex;
use reqwest::Client;
use std::{format, fs, path::PathBuf, sync::Arc};
use tokio::{
    select,
    sync::mpsc::{self, channel},
};
use tokio_tungstenite::{connect_async, tungstenite::Message};
use whatssock_desktop::{
    authentication::auth::{create_hwid_key, decrypt_bytes},
    ApplicationContext, HttpClient, Route, COOKIE_SAVE_PATH,
};
use whatssock_lib::{client::UserInformation, UserSession};

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() -> anyhow::Result<()> {
    // Initalize a local file for in the user's folder
    let path = PathBuf::from(format!(
        "{}/.whatssock",
        std::env::var("USERPROFILE").unwrap()
    ));

    // Only attempt to create the folder if it doesnt exist yet
    if let Err(_err) = std::fs::read_dir(&path) {
        std::fs::create_dir(path)?;
    }

    dioxus::launch(App);

    Ok(())
}

#[component]
fn App() -> Element {
    let toast = use_context_provider(|| Signal::new(ToastManager::default()));

    rsx! {
        ToastFrame {
            manager: toast,
        }
        Router::<Route> {}
        {
            init_application()
        }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
    }
}

/// Initalize application:
/// Send the stored session cookie to the server on startup, and automaticly log in if we have a valid cookie.
#[component]
fn init_application() -> Element {
    // Bind sender to REST API
    let server_sender = Arc::new(Mutex::new(HttpClient::new(Client::new(), {
        #[cfg(debug_assertions)]
        {
            String::from("http://[::1]:3004")
        }
        #[cfg(not(debug_assertions))]
        {
            String::from("http://whatssock.com")
        }
    })));

    let (websocket_sender, mut websocket_receiver) = channel::<()>(255);
    let (remote_sender, remote_receiver) = mpsc::channel::<Message>(255);

    tokio::spawn(async move {
        let (ws_socket, response) = connect_async({
            #[cfg(debug_assertions)]
            {
                String::from("ws://[::1]:3004")
            }
            #[cfg(not(debug_assertions))]
            {
                String::from("ws://whatssock.com")
            }
        })
        .await
        .unwrap();

        let (mut write, mut read) = ws_socket.split();

        loop {
            select! {
                // This poll is going to wait until it receives a message from the client to send out a message.
                // It uses a mpsc to receive the messages from various points of the code.
                sendable_value = websocket_receiver.recv() => {
                    match sendable_value {
                        Some(message) => {
                            // Handle sending out the message through the websocket
                            write.send(todo!()).await.unwrap();
                        },
                        None => {
                            error!("Websocket receiver handler channel closed. Websocket closed.");
                            break;
                        },
                    }
                },
                received_value = read.next() => {
                    if let Some(message) = received_value {
                        match message {
                            Ok(message) => {
                                remote_sender.send(message).await.unwrap();
                            },
                            Err(err) => {

                            },
                        }
                    }
                }
            }
        }
    });

    let server_sender_clone = server_sender.clone();
    let mut log_res: Signal<Option<(UserSession, UserInformation)>> = use_signal(|| None);

    use_root_context::<Signal<Option<(UserSession, UserInformation)>>>(|| Signal::new(None));
    use_root_context(|| ApplicationContext {
        http_client: server_sender_clone,
        websocket_client_out: websocket_sender,
        websocket_client_in: Arc::new(Mutex::new(remote_receiver)),
    });

    if let Ok(encrypted_bytes) = fs::read(&*COOKIE_SAVE_PATH) {
        // We should decrypt the bytes so that we can get the cookie
        match decrypt_bytes(encrypted_bytes, create_hwid_key().unwrap_or_default()) {
            Ok(user_session) => {
                let user_session = user_session.clone();
                let server_sender = server_sender.clone();

                spawn(async move {
                    // Verify user session with server
                    match server_sender
                        .lock()
                        .verify_user_session(user_session.clone())
                        .await
                    {
                        Ok(response) => {
                            let user_information = serde_json::from_str::<UserInformation>(
                                &response.text().await.unwrap(),
                            )
                            .unwrap();

                            log_res.set(Some((user_session, user_information)));
                        }
                        Err(err) => {
                            error!("{err}");
                        }
                    };
                });
            }
            Err(err) => {
                error!("Error occured while trying to decrypt session cookie: {err}")
            }
        };
    }

    use_effect(move || {
        if let Some(active_session) = (*log_res.read()).clone() {
            let mut session = use_context::<Signal<Option<(UserSession, UserInformation)>>>();
            session.set(Some(active_session));
        }
    });

    rsx!()
}
