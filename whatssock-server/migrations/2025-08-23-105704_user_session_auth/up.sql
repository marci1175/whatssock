-- Your SQL goes here
CREATE TABLE user_session_auth (
    token_id SERIAL PRIMARY KEY,
    user_id INT NOT NULL,
    session_token BYTEA NOT NULL,
    encryption_key BYTEA NOT NULL
);