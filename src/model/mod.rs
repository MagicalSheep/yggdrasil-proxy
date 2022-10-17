pub mod reply;
pub mod request;
pub mod errors;

use std::collections::HashMap;
use reqwest::StatusCode;
use rsa::{RsaPrivateKey, RsaPublicKey};
use rsa::pkcs8::{EncodePrivateKey, EncodePublicKey};
use serde_derive::{Deserialize, Serialize};
use crate::{IMPLEMENTATION_NAME, LineEnding, PUBLIC_KEY, VERSION};
use crate::model::errors::CustomError;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessClaims {
    pub exp: i64,
    pub tokens: HashMap<String, String>,
    pub uuids: HashMap<String, String>,
    pub selected: HashMap<String, bool>,
    pub selected_uuid: Option<String>, // for proxy server
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyPair {
    #[serde(rename = "privateKey")]
    pub private_key: String,
    #[serde(rename = "publicKey")]
    pub public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub name: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub signature: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub properties: Vec<Property>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<Vec<Property>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaLinksProperty {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub register: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaProperty {
    #[serde(rename = "serverName", skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    #[serde(rename = "implementationName", skip_serializing_if = "Option::is_none")]
    pub implementation_name: Option<String>,
    #[serde(rename = "implementationVersion", skip_serializing_if = "Option::is_none")]
    pub implementation_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<MetaLinksProperty>,
    #[serde(rename = "feature.non_email_login", skip_serializing_if = "Option::is_none")]
    pub non_email_login: Option<bool>,
    #[serde(rename = "feature.legacy_skin_api", skip_serializing_if = "Option::is_none")]
    pub legacy_skin_api: Option<bool>,
    #[serde(rename = "feature.no_mojang_namespace", skip_serializing_if = "Option::is_none")]
    pub no_mojang_namespace: Option<bool>,
    #[serde(rename = "feature.enable_mojang_anti_features", skip_serializing_if = "Option::is_none")]
    pub enable_mojang_anti_features: Option<bool>,
    #[serde(rename = "feature.enable_profile_key", skip_serializing_if = "Option::is_none")]
    pub enable_profile_key: Option<bool>,
    #[serde(rename = "feature.username_check", skip_serializing_if = "Option::is_none")]
    pub username_check: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Meta {
    pub meta: MetaProperty,
    #[serde(rename = "skinDomains")]
    pub skin_domains: Vec<String>,
    #[serde(rename = "signaturePublickey")]
    pub signature_public_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigMeta {
    #[serde(rename = "serverName", skip_serializing_if = "Option::is_none")]
    pub server_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub links: Option<MetaLinksProperty>,
    #[serde(rename = "feature.non_email_login", skip_serializing_if = "Option::is_none")]
    pub non_email_login: Option<bool>,
    #[serde(rename = "feature.legacy_skin_api", skip_serializing_if = "Option::is_none")]
    pub legacy_skin_api: Option<bool>,
    #[serde(rename = "feature.no_mojang_namespace", skip_serializing_if = "Option::is_none")]
    pub no_mojang_namespace: Option<bool>,
    #[serde(rename = "feature.enable_mojang_anti_features", skip_serializing_if = "Option::is_none")]
    pub enable_mojang_anti_features: Option<bool>,
    #[serde(rename = "feature.enable_profile_key", skip_serializing_if = "Option::is_none")]
    pub enable_profile_key: Option<bool>,
    #[serde(rename = "feature.username_check", skip_serializing_if = "Option::is_none")]
    pub username_check: Option<bool>,
    #[serde(rename = "skinDomains")]
    pub skin_domains: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub meta: ConfigMeta,
    #[serde(rename = "dataSource")]
    pub data_source: String,
    pub secret: String,
    pub address: String,
    pub port: u16,
    pub backends: HashMap<String, String>,
    pub main: String,
    #[serde(rename = "enableMasterSlaveMode")]
    pub enable_master_slave_mode: bool,
}

impl From<ConfigMeta> for MetaProperty {
    fn from(config_meta: ConfigMeta) -> Self {
        MetaProperty {
            server_name: config_meta.server_name,
            implementation_name: Some(IMPLEMENTATION_NAME.to_string()),
            implementation_version: Some(VERSION.to_string()),
            links: config_meta.links,
            non_email_login: config_meta.non_email_login,
            legacy_skin_api: config_meta.legacy_skin_api,
            no_mojang_namespace: config_meta.no_mojang_namespace,
            enable_mojang_anti_features: config_meta.enable_mojang_anti_features,
            enable_profile_key: config_meta.enable_profile_key,
            username_check: config_meta.username_check,
        }
    }
}

impl From<&Config> for Meta {
    fn from(config: &Config) -> Self {
        Meta {
            meta: config.meta.clone().into(),
            skin_domains: config.meta.skin_domains.clone(),
            signature_public_key: PUBLIC_KEY.clone(),
        }
    }
}

impl Config {
    pub fn new() -> Config {
        let mut backends = HashMap::new();
        backends.insert("ls".to_string(), "https://littleskin.cn/api/yggdrasil".to_string());
        backends.insert("example".to_string(), "https://example.com/api/yggdrasil".to_string());
        let mut skin_domains = vec![];
        skin_domains.push("littleskin.cn".to_string());
        skin_domains.push("skin.prinzeugen.net".to_string());
        skin_domains.push("example.com".to_string());
        Config {
            meta: ConfigMeta {
                server_name: Some("Union Authenticate Server".to_string()),
                links: Some(MetaLinksProperty {
                    homepage: Some("https://example.com".to_string()),
                    register: Some("https://example.com/auth/register".to_string()),
                }),
                non_email_login: Some(true),
                legacy_skin_api: Some(false),
                no_mojang_namespace: Some(false),
                enable_mojang_anti_features: Some(false),
                enable_profile_key: Some(true),
                username_check: Some(false),
                skin_domains,
            },
            data_source: "mysql://root:password@localhost/database".to_string(),
            secret: "example-token-secret".to_string(),
            address: "0.0.0.0".to_string(),
            port: 8080,
            backends,
            main: "ls".to_string(),
            enable_master_slave_mode: true
        }
    }
}

impl KeyPair {
    pub fn new() -> Result<KeyPair, CustomError> {
        let mut rng = rand::thread_rng();
        let private_key = match RsaPrivateKey::new(&mut rng, 2048) {
            Ok(res) => { res }
            Err(err) => {
                return Err(
                    CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err)));
            }
        };
        let public_key = match RsaPublicKey::from(private_key.clone()).to_public_key_der() {
            Ok(res) => { res }
            Err(err) => {
                return Err(
                    CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err)));
            }
        };
        let public_key = match public_key.to_pem("RSA PUBLIC KEY", LineEnding::default()) {
            Ok(res) => { res }
            Err(err) => {
                return Err(
                    CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err)));
            }
        };
        let private_key = match private_key.to_pkcs8_der() {
            Ok(res) => { res }
            Err(err) => {
                return Err(
                    CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err)));
            }
        };
        let private_key = match private_key.to_pem("RSA PRIVATE KEY", LineEnding::default()) {
            Ok(res) => { res.to_string() }
            Err(err) => {
                return Err(
                    CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err)));
            }
        };
        Ok(KeyPair {
            private_key,
            public_key,
        })
    }
}