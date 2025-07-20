#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LoginResponse {
    pub user_id: i32,
    pub session_token: [u8; 32],
    pub chatrooms_joined: Vec<Option<i32>>,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct LogoutResponse {}
