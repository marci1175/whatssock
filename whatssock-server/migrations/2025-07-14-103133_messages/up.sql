CREATE TABLE messages (
    id SERIAL PRIMARY KEY,
    owner_user_id INT NOT NULL,
    replying_to_msg INT,
    parent_chatroom_id INT NOT NULL,
    -- The message will always be stored serialized in a (rmp_serde) byte format
    raw_message BYTEA NOT NULL,
    send_date TIMESTAMP NOT NULL DEFAULT clock_timestamp()
);