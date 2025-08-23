use crate::api::user_account_control::users::dsl::users;
use crate::models::{
    NewUserAccount, NewUserSession, UpdateLastMessage, UserAccountEntry, UserSessionEntry,
};
use crate::schema::chatrooms::dsl::chatrooms;
use crate::schema::user_session_auth::dsl::user_session_auth;
use crate::schema::user_session_auth::{session_token, user_id};
use crate::schema::users::{id, passw, username};
use crate::{
    ServerState,
    schema::{self, *},
};
use axum::{Json, extract::State, http::StatusCode};
use diesel::dsl::{count_star, exists};
use diesel::query_dsl::methods::{FilterDsl, SelectDsl};
use diesel::{
    ExpressionMethods, QueryResult, RunQueryDsl, SelectableHelper, delete, select, update,
};
use log::error;
use rand::{Rng, rng};
use whatssock_lib::{UserSession, UserSessionSecure};
use whatssock_lib::client::{LoginRequest, RegisterRequest, UserSessionInformation};
use whatssock_lib::server::{LoginResponse, LoginResponseSecure, LogoutResponse};

pub async fn fetch_login(
    State(state): State<ServerState>,
    Json(information): Json<LoginRequest>,
) -> Result<Json<LoginResponseSecure>, StatusCode> {
    let mut pg_connection = state.pg_pool.get().map_err(|err| {
        error!(
            "An error occured while fetching login information from db: {}",
            err
        );

        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let user_account = users
        .filter(username.eq(information.username.clone()))
        .filter(passw.eq(information.password))
        .select(UserAccountEntry::as_select())
        .get_result(&mut pg_connection)
        .map_err(|err| {
            error!(
                "An error occured while searching for the user's account: {}",
                err
            );

            StatusCode::NOT_FOUND
        })?;

    // Issue a new session token for future logins
    let session_cookie_token = generate_random_secure_key();
    
    // Issue a new encryption key for communication between the client and the server
    let encryption_key = generate_random_secure_key();

    let user_session_count = user_session_auth
        .filter(user_id.eq(user_account.id))
        .select(count_star())
        .first::<i64>(&mut pg_connection)
        .map_err(|err| {
            error!(
                "An error occured while searching for the user's session token: {}",
                err
            );

            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Check if there are any existing user sessions
    // If there arent this means some sort of issue has occured, thus the session has been invalidated or deleted.
    if user_session_count != 0 {
        // Search up a session token for the user, if it exists update it
        diesel::update(user_session_auth)
            .filter(user_id.eq(user_account.id))
            .set(&NewUserSession {
                user_id: user_account.id,
                // Issue a new session token for future logins
                session_token: session_cookie_token.clone().to_vec(),
                encryption_key: encryption_key.clone().to_vec()
            })
            .get_result::<UserSessionEntry>(&mut pg_connection)
            .map_err(|err| {
                error!(
                    "An error occured while searching for the user's session token: {}",
                    err
                );

                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    } else {
        diesel::insert_into(user_session_auth)
            .values(&NewUserSession {
                user_id: user_account.id,
                // Issue a new session token for future logins
                session_token: session_cookie_token.clone().to_vec(),
                encryption_key: encryption_key.clone().to_vec()
            })
            .get_result::<UserSessionEntry>(&mut pg_connection)
            .map_err(|err| {
                error!(
                    "An error occured while fetching login information from db: {}",
                    err
                );
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    Ok(Json(LoginResponseSecure {
        user_information: UserSessionInformation {
            username: user_account.username,
            chatrooms_joined: user_account.chatrooms_joined,
            user_id: user_account.id,
        },
        user_session_secure: UserSessionSecure {
            user_id: user_account.id,
            session_token: session_cookie_token,
            encryption_key: encryption_key,
        },
    }))
}

pub async fn register_user(
    State(state): State<ServerState>,
    Json(information): Json<RegisterRequest>,
) -> Result<Json<LoginResponseSecure>, StatusCode> {
    let mut pg_connection = state.pg_pool.get().map_err(|err| {
        error!(
            "An error occured while fetching login information from db: {}",
            err
        );

        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Check if there are existing users with this name.
    let user_count = users
        .filter(username.eq(information.username.clone()))
        .select(count_star())
        .first::<i64>(&mut pg_connection)
        .map_err(|err| {
            error!(
                "An error occured while fetching login information from db: {}",
                err
            );
            StatusCode::REQUEST_TIMEOUT
        })?;

    if user_count != 0 {
        return Err(StatusCode::FOUND);
    }

    // Insert the user's register information into the DB
    let user_account = diesel::insert_into(users)
        .values(&NewUserAccount {
            username: information.username.clone(),
            passw: information.password,
            chatrooms_joined: vec![],
            email: information.email,
        })
        .get_result::<UserAccountEntry>(&mut pg_connection)
        .map_err(|err| {
            error!(
                "An error occured while fetching login information from db: {}",
                err
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Issue a new session token for future logins
    let session_cookie_token = generate_random_secure_key();
    
    // Issue a new encryption key for communication between the client and the server
    let encryption_key = generate_random_secure_key();

    // Store the session token in the db, there is no way of having another session token for this user as we have just created it.
    diesel::insert_into(user_session_auth)
        .values(&NewUserSession {
            user_id: user_account.id,
            // Issue a new session token for future logins
            session_token: session_cookie_token.clone().to_vec(),
            // Issue a new encryption key for communication between the client and the server
            encryption_key: encryption_key.clone().to_vec(),
        })
        .get_result::<UserSessionEntry>(&mut pg_connection)
        .map_err(|err| {
            error!(
                "An error occured while fetching login information from db: {}",
                err
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(LoginResponseSecure {
        user_session_secure: UserSessionSecure {
            user_id: user_account.id,
            session_token: session_cookie_token,
            encryption_key: encryption_key,
        },
        user_information: UserSessionInformation {
            username: user_account.username,
            chatrooms_joined: user_account.chatrooms_joined,
            user_id: user_account.id,
        },
    }))
}

pub async fn fetch_user_information_from_session(
    State(state): State<ServerState>,
    Json(user_session): Json<UserSession>,
) -> Result<Json<UserSessionInformation>, StatusCode> {
    // Get a db connection from the pool
    let mut pg_connection = state.pg_pool.get().map_err(|err| {
        error!(
            "An error occured while fetching login information from db: {}",
            err
        );

        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Verify user session, this will return an error if the session is not found in the DB.
    verify_user_session(&user_session, &mut pg_connection)?;

    let user_account = users
        .filter(id.eq(user_session.user_id))
        .select(UserAccountEntry::as_select())
        .first::<UserAccountEntry>(&mut pg_connection)
        .map_err(|err| {
            error!(
                "An error occured while fetching user session information from db: {}",
                err
            );
            StatusCode::REQUEST_TIMEOUT
        })?;

    Ok(Json(UserSessionInformation {
        username: user_account.username,
        chatrooms_joined: user_account.chatrooms_joined,
        user_id: user_account.id,
    }))
}

pub async fn handle_logout_request(
    State(state): State<ServerState>,
    Json(session_cookie): Json<UserSession>,
) -> Result<Json<LogoutResponse>, StatusCode> {
    // Get a db connection from the pool
    let mut pg_connection = state.pg_pool.get().map_err(|err| {
        error!(
            "An error occured while fetching login information from db: {}",
            err
        );

        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    match delete(user_session_auth.filter(session_token.eq(session_cookie.session_token)))
        .execute(&mut pg_connection)
    {
        Ok(r_affected) => {
            dbg!(r_affected);
        }
        Err(err) => {
            error!("{err}");

            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    }

    Ok(Json(LogoutResponse {}))
}

pub fn generate_random_secure_key() -> [u8; 32] {
    let mut rng = rng();

    let mut custom_identifier = [0_u8; 32];

    rng.fill(&mut custom_identifier);

    custom_identifier
}

/// Checks the [`UserSession`] passed in.
/// The function checks for the session token and the id. If either one of them dont match, it will return [`StatusCode::FORBIDDEN`].
pub fn verify_user_session(
    // The UserSession which is checked
    user_session: &UserSession,
    // A valid DB connection
    pg_connection: &mut r2d2::PooledConnection<
        diesel::r2d2::ConnectionManager<diesel::PgConnection>,
    >,
) -> Result<UserSessionEntry, StatusCode> {
    let user_session: UserSessionEntry = 
        user_session_auth
            .filter(schema::user_session_auth::user_id.eq(user_session.user_id))
            .filter(schema::user_session_auth::session_token.eq(user_session.session_token))
            .select(UserSessionEntry::as_select())
            .get_result::<UserSessionEntry>(pg_connection)
            .map_err(|_err| {
                StatusCode::FORBIDDEN
            })?;

    Ok(user_session)
}

pub fn lookup_joined_chatrooms(
    // A valid DB connection
    pg_connection: &mut r2d2::PooledConnection<
        diesel::r2d2::ConnectionManager<diesel::PgConnection>,
    >,
    user_uid: i32,
) -> QueryResult<Vec<Option<i32>>> {
    users
        .filter(id.eq(user_uid))
        .select(UserAccountEntry::as_select())
        .get_result(pg_connection)
        .map(|entry| entry.chatrooms_joined)
}

pub fn update_chatroom_last_msg(
    message_id: i32,
    chatroom_id: i32,
    pg_connection: &mut r2d2::PooledConnection<
        diesel::r2d2::ConnectionManager<diesel::PgConnection>,
    >,
) -> Result<usize, diesel::result::Error> {
    update(chatrooms.filter(schema::chatrooms::id.eq(chatroom_id)))
        .set(&UpdateLastMessage {
            last_message_id: message_id,
        })
        .execute(pg_connection)
}
