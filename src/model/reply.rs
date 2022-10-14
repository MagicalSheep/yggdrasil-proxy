use crate::model::{KeyPair, Profile, User};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthenticateReply {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "clientToken")]
    pub client_token: Option<String>,
    #[serde(rename = "availableProfiles")]
    pub available_profiles: Vec<Profile>,
    #[serde(rename = "selectedProfile", skip_serializing_if = "Option::is_none")]
    pub selected_profile: Option<Profile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RefreshReply {
    #[serde(rename = "accessToken")]
    pub access_token: String,
    #[serde(rename = "clientToken")]
    pub client_token: Option<String>,
    #[serde(rename = "selectedProfile", skip_serializing_if = "Option::is_none")]
    pub selected_profile: Option<Profile>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user: Option<User>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CertificatesReply {
    #[serde(rename = "expiresAt")]
    pub expires_at: String,
    #[serde(rename = "keyPair")]
    pub key_pair: KeyPair,
    #[serde(rename = "publicKeySignature")]
    pub public_key_signature: String,
    #[serde(rename = "publicKeySignatureV2")]
    pub public_key_signature_v2: String,
    #[serde(rename = "refreshedAfter")]
    pub refreshed_after: String
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorReply {
    pub error: String,
    #[serde(rename = "errorMessage")]
    pub error_message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cause: Option<String>,
}