use std::{fmt::Display, sync::Arc};

use crate::{
    authentication::auth::{deserialize_into_user_session, store_user_session_on_disk},
    HttpClient, Route, COOKIE_SAVE_PATH,
};
use dioxus::{logger::tracing, prelude::*};
use parking_lot::Mutex;
use whatssock_lib::{client::UserInformation, UserSession};

enum AttemptResult {
    Attempted(String),
    Succeeded(String),
    Failed(String),
}

impl Display for AttemptResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            AttemptResult::Attempted(inner) => inner,
            AttemptResult::Succeeded(inner) => inner,
            AttemptResult::Failed(inner) => inner,
        })
    }
}

#[component]
pub fn Login() -> Element {
    let navigator = use_navigator();
    let valid_token_redirect = use_context::<Signal<Option<(UserSession, UserInformation)>>>();

    let client = use_context::<Arc<Mutex<HttpClient>>>();
    let mut user_session_login: Signal<Option<(UserSession, UserInformation)>, SyncStorage> =
        use_signal_sync(|| None);
    let mut log_res: Signal<Option<AttemptResult>> = use_signal(|| None);
    let mut username = use_signal(String::new);
    let mut password = use_signal(String::new);
    rsx! {
        {
            if let Some(valid_session) = valid_token_redirect.read().clone() {
                // Add the UserSession to the context
                use_root_context(|| valid_session);

                navigator.push(Route::MainPage {  });
            }
        }

        div {
            id: "login_page_container",
            div {
                id: "main_title",
                class: "title",
                "Whatssock"
            }

            footer {
                id: "main_title_footer",
                "Wassup? Interact with others now."
            }

            div {
                id: "user_input_fields",

                div {
                    id: "username_field",
                    input {
                        oninput: move |event| username.set(event.value()),
                        placeholder: "Username",
                    }
                }

                div {
                    id: "password_field",
                    input {
                        oninput: move |event| password.set(event.value()),
                        placeholder: "Password",
                        r#type: "password",
                    }
                }

                button { id: "ui_button", class: "button", onclick: move |_| {
                    // Update state
                    log_res.set(Some(AttemptResult::Attempted("Logging in...".to_string())));

                    let client = client.clone();

                    // Spawn async task
                    spawn(async move {
                        let client = client.lock();
                        match client.fetch_login(username.to_string(), password.to_string()).await {
                            Ok(response) => {
                                let user_session = deserialize_into_user_session(response.text().await.unwrap()).unwrap();

                                tracing::info!("{:?}", &user_session);

                                // Verify user session with server
                                match client
                                    .verify_user_session(user_session.clone())
                                    .await
                                {
                                    Ok(response) => {
                                        let user_information = serde_json::from_str::<UserInformation>(&response.text().await.unwrap()).unwrap();

                                        user_session_login.set(Some((user_session.clone(), user_information)));
                                    }
                                    Err(err) => {
                                        tracing::error!("Error occured when verifying session token: {}", err.to_string());

                                        // Update state
                                        log_res.set(Some(AttemptResult::Failed(err.to_string())));
                                    }
                                };


                                store_user_session_on_disk(&user_session, (*COOKIE_SAVE_PATH).clone()).unwrap();

                                // Update state
                                log_res.set(Some(AttemptResult::Succeeded("Login Successful! Redirecting....".to_string())));
                            },
                            Err(err) => {
                                tracing::error!("Error occured when logging in: {}", err.to_string());

                                // Update state
                                log_res.set(Some(AttemptResult::Failed(err.to_string())));
                            },
                        }
                    });
                }, "Login" }

                li {
                    Link {
                        to: crate::Route::Register {  },
                        "Not registered?",
                    },
                }

                // Check if there is an existing error message
                div {
                    id: "login_result",
                    {
                        if let Some(login_result) = &*log_res.read() {
                            // Display the result
                            match login_result {
                                AttemptResult::Attempted(inner) => {
                                    rsx! {
                                        div {
                                            id: "attempted",
                                            {
                                                inner.to_string()
                                            }
                                        }
                                    }
                                },
                                AttemptResult::Succeeded(inner) => {
                                    rsx! {
                                        div {
                                            id: "succeeded",
                                            {
                                                inner.to_string()
                                            }
                                        }
                                    }
                                },
                                AttemptResult::Failed(inner) => {
                                    rsx! {
                                        div {
                                            id: "failed",
                                            {
                                                inner.to_string()
                                            }
                                        }
                                    }
                                },
                            }
                        }
                        else {
                            rsx!()
                        }
                    }
                }

                // Check if we have logged in
                {
                    if let Some(user_session) = user_session_login.read().clone() {
                        provide_root_context(user_session);
                        navigator.push(crate::Route::MainPage { });
                    }
                }
            }
        }

        ErrorBoundary {
            handle_error: |_| {
                rsx! {
                    div {
                        "Oops, we encountered an error. Please report this to the developer of this application"
                    }
                }
            }
        }
    }
}
