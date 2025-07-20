CREATE TABLE chatrooms (
    id SERIAL PRIMARY KEY,
    chatroom_id VARCHAR NOT NULL,
    chatroom_name VARCHAR NOT NULL,
    chatroom_password VARCHAR,
    participants INT[] NOT NULL,
    is_direct_message BOOLEAN NOT NULL,
    last_message_id INT
);