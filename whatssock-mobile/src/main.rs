use dioxus::{logger::tracing::error, prelude::*};
use dioxus_toast::{ToastFrame, ToastManager};
use futures_util::StreamExt;
use parking_lot::Mutex;
use reqwest::Client;
use std::{format, fs, path::PathBuf, sync::Arc};
use whatssock_mobile::{
    api_requests::init_websocket_connection,
    authentication::auth::{create_hwid_key, decrypt_bytes},
    HttpClient, Route, COOKIE_SAVE_PATH,
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

    let server_sender_clone: Arc<parking_lot::lock_api::Mutex<parking_lot::RawMutex, HttpClient>> =
        server_sender.clone();

    use_root_context(|| server_sender_clone);
    let mut log_res: Signal<Option<(UserSession, UserInformation)>> = use_signal(|| None);
    use_root_context::<Signal<Option<(UserSession, UserInformation)>>>(|| Signal::new(None));

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
        if let Some((usr_session, user_info)) = (*log_res.read()).clone() {
            let mut session = use_context::<Signal<Option<(UserSession, UserInformation)>>>();
            session.set(Some((usr_session.clone(), user_info)));
            provide_root_context({
                let (sender, reciever) = init_websocket_connection(usr_session.clone());

                (sender, Arc::new(Mutex::new(reciever)))
            });
        }
    });

    rsx!()
}
