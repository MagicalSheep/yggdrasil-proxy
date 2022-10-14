use base64::encode;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use log::debug;
use rsa::pkcs1v15::SigningKey;
use sha1::Sha1;
use signature::{Signature, Signer};
use warp::http::StatusCode;
use crate::model::AccessClaims;
use crate::model::errors::CustomError;
use crate::{CONFIG, PRIVATE_KEY};

/// Create a new proxy server access token
pub fn create_token(claims: &AccessClaims) -> String {
    jsonwebtoken::encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(CONFIG.secret.as_ref()),
    ).unwrap()
}

/// Decode access token provided by proxy server
pub fn decode_token(token: &str) -> Result<AccessClaims, CustomError> {
    match jsonwebtoken::decode::<AccessClaims>(
        token,
        &DecodingKey::from_secret(CONFIG.secret.as_ref()),
        &Validation::new(Algorithm::HS256)) {
        Ok(val) => {
            debug!("Decode token successfully: {:#?}", val.claims);
            Ok(val.claims)
        }
        Err(err) => {
            debug!("Decode token error: {}", err);
            Err(CustomError::ForbiddenOperationException(StatusCode::FORBIDDEN, "Invalid token.".to_string()))
        }
    }
}

/// Get a signature using the proxy server private key, and encode it with Base64
pub fn signature(content: String) -> String {
    let private_key = &*PRIVATE_KEY;
    let signing_key = SigningKey::<Sha1>::new_with_prefix(private_key.clone());
    let sign = signing_key.sign(content.as_bytes());
    encode(sign.as_bytes())
}