CREATE TABLE messages (
    id SERIAL PRIMARY KEY,
    parent_chatroom_id INT NOT NULL,
    owner_user_id INT NOT NULL,
    send_date TIMESTAMP NOT NULL DEFAULT clock_timestamp(),
    -- The message will always be stored serialized in a (rmp_serde) byte format
    raw_message BYTEA NOT NULL
);