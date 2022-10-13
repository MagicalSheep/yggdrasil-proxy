use crate::model::Profile;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    pub name: String,
    pub version: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthenticateRequest {
    pub username: String,
    pub password: String,
    #[serde(rename = "clientToken")]
    pub client_token: Option<String>,
    #[serde(rename = "requestUser")]
    pub request_user: bool,
    pub agent: Agent,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefreshRequest {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "clientToken")]
    pub client_token: Option<String>,
    #[serde(rename = "requestUser")]
    pub request_user: bool,
    #[serde(rename = "selectedProfile", skip_serializing_if = "Option::is_none")]
    pub selected_profile: Option<Profile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateRequest {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "clientToken")]
    pub client_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogoutRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinRequest {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "selectedProfile")]
    pub selected_profile: String,
    #[serde(rename = "serverId")]
    pub server_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JoinQuery {
    pub username: String,
    #[serde(rename = "serverId")]
    pub server_id: String,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileQuery {
    pub unsigned: Option<bool>,
}