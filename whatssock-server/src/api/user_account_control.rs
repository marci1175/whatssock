use crate::api::user_account_control::users::dsl::users;
use crate::models::{NewUserAccount, NewUserSession, UserAccountEntry, UserSessionEntry};
use crate::schema::user_signin_tokens::dsl::user_signin_tokens;
use crate::schema::user_signin_tokens::{session_token, user_id};
use crate::schema::users::{id, passw, username};
use crate::{
    ServerState,
    schema::{self, *},
};
use axum::{Json, extract::State, http::StatusCode};
use diesel::dsl::count_star;
use diesel::query_dsl::methods::{FilterDsl, SelectDsl};
use diesel::{ExpressionMethods, RunQueryDsl, SelectableHelper, delete};
use log::error;
use rand::{Rng, rng};
use whatssock_lib::UserSession;
use whatssock_lib::client::{LoginRequest, RegisterRequest, UserInformation};
use whatssock_lib::server::{LoginResponse, LogoutResponse};

pub async fn fetch_login(
    State(state): State<ServerState>,
    Json(information): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
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
    let session_cookie_token = generate_session_token();

    let user_session_count = user_signin_tokens
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
        diesel::update(user_signin_tokens)
            .filter(user_id.eq(user_account.id))
            .set(&NewUserSession {
                user_id: user_account.id,
                session_token: session_cookie_token.clone().to_vec(),
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
        diesel::insert_into(user_signin_tokens)
            .values(&NewUserSession {
                user_id: user_account.id,
                session_token: session_cookie_token.clone().to_vec(),
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

    Ok(Json(LoginResponse {
        user_id: user_account.id,
        session_token: session_cookie_token,
        chatrooms_joined: user_account.chatrooms_joined,
    }))
}

pub async fn register_user(
    State(state): State<ServerState>,
    Json(information): Json<RegisterRequest>,
) -> Result<Json<LoginResponse>, StatusCode> {
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
    let session_cookie_token = generate_session_token();

    // Store the session token in the db, there is no way of having another session token for this user as we have just created it.
    diesel::insert_into(user_signin_tokens)
        .values(&NewUserSession {
            user_id: user_account.id,
            session_token: session_cookie_token.clone().to_vec(),
        })
        .get_result::<UserSessionEntry>(&mut pg_connection)
        .map_err(|err| {
            error!(
                "An error occured while fetching login information from db: {}",
                err
            );
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(LoginResponse {
        user_id: user_account.id,
        session_token: session_cookie_token,
        chatrooms_joined: user_account.chatrooms_joined,
    }))
}

pub async fn fetch_session_token(
    State(state): State<ServerState>,
    Json(user_session): Json<UserSession>,
) -> Result<Json<UserInformation>, StatusCode> {
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

    Ok(Json(UserInformation {
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

    match delete(user_signin_tokens.filter(session_token.eq(session_cookie.session_token)))
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

pub fn generate_session_token() -> [u8; 32] {
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
) -> Result<(), StatusCode> {
    let matching_user_tokens = user_signin_tokens
        .filter(schema::user_signin_tokens::user_id.eq(user_session.user_id))
        .filter(schema::user_signin_tokens::session_token.eq(user_session.session_token))
        .select(count_star())
        .get_result::<i64>(pg_connection)
        .map_err(|err| {
            error!(
                "An error occured while verifying login information from db: {}",
                err
            );

            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    if matching_user_tokens != 1 {
        return Err(StatusCode::FORBIDDEN);
    }

    Ok(())
}
