#[derive(serde::Serialize, serde::Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
    pub email: String,
}

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone)]
pub struct UserInformation {
    pub username: String,
    pub chatrooms_joined: Vec<Option<i32>>,
    pub user_id: i32,
}
