-- Your SQL goes here
CREATE TABLE users (
    id SERIAL PRIMARY KEY,
    username VARCHAR NOT NULL,
    passw VARCHAR NOT NULL,
    email VARCHAR NOT NULL,
    chatrooms_joined INT[] NOT NULL,
    created_at DATE NOT NULL DEFAULT NOW()
);