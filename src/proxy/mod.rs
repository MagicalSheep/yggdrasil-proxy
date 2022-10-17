pub mod proxy;
pub mod pre_proxy;

use base64::decode;
use log::debug;
use reqwest::{Client, StatusCode};
use rsa::pkcs1v15::VerifyingKey;
use rsa::pkcs8::DecodePublicKey;
use rsa::RsaPublicKey;
use sea_orm::ActiveValue;
use sha1::Sha1;
use signature::Verifier;
use uuid::Uuid;
use crate::CONFIG;
use crate::model::errors::CustomError;
use crate::model::{Meta, Profile, Property};
use crate::repository::{find_by_backend_and_uuid, save_profile};
use crate::utils::signature;

/// Validate signature from backend server
async fn validate_sign(src_backend: &str, sign: &str, content: &str) -> bool {
    // sign is a base64 encoded str, should decode it firstly
    let sign = match decode(sign) {
        Ok(res) => { res }
        Err(_) => { return false; }
    };
    // get backend server rsa public key
    let url = match CONFIG.backends.get(src_backend) {
        None => { return false; }
        Some(u) => { u }
    };
    let resp = match Client::new().get(url).send().await {
        Ok(res) => { res }
        Err(_) => { return false; }
    };
    let public_key_str = match resp.json::<Meta>().await {
        Ok(res) => { res.signature_public_key }
        Err(_) => { return false; }
    };
    // build public key
    let public_key = match RsaPublicKey::from_public_key_pem(&public_key_str) {
        Ok(res) => { res }
        Err(_) => { return false; }
    };
    // verify signature
    let verifying_key = VerifyingKey::<Sha1>::new_with_prefix(public_key);
    let sign = rsa::pkcs1v15::Signature::from(sign);
    // content is already a base64 encoded str
    match verifying_key.verify(content.as_ref(), &sign) {
        Ok(_) => {
            debug!("Validate signature successfully, content: {}", content);
            true
        }
        Err(_) => {
            debug!("Validate signature failed, content: {}", content);
            false
        }
    }
}

/// Validate and resign signature for properties.
///
/// It will validate the signature from backend server before resign it.
/// An invalid signature will be skipped to resign so that
/// it can cause a validation fail in Minecraft client.
async fn re_signature(src_backend: &str, properties: Option<Vec<Property>>) -> Option<Vec<Property>> {
    if let None = properties { return None; }
    let properties = properties.unwrap();
    let mut ret = vec![];
    for property in properties {
        let p = match &property.signature {
            None => { property }
            Some(sign) => {
                // if validate failed, skip it
                if !validate_sign(src_backend, &sign, &property.value).await { property } else {
                    let resign = signature(property.value.clone());
                    debug!("Resign signature for content {} is: {}", &property.value, &resign);
                    Property {
                        name: property.name.clone(),
                        value: property.value.clone(),
                        signature: Some(resign),
                    }
                }
            }
        };
        ret.push(p);
    }
    Some(ret)
}

/// Translate the profile from a specific backend server into the profile that the proxy server controls.
///
/// - Profile name will be renamed to `{backend_server_id}_{username}`.
/// - Profile UUID will be replaced by a new version 4 UUID.
/// - Profile properties will be resigned signature using the proxy server private key
/// for all properties that the signature exists.
///
/// However, if src_backend is the main server, this function will return its origin UUID and name value.
/// Although its value in the database has been changed.
pub async fn translate(src_backend: &str, profile: Profile) -> Result<Profile, CustomError> {
    let is_need_to_trans = !CONFIG.enable_master_slave_mode || CONFIG.main.ne(src_backend);

    // backend server id and uuid can decide a profile
    // note: backend server id and name cannot decide a profile because user can rename profile
    let res = find_by_backend_and_uuid(src_backend, &profile.id).await;
    if let Err(err) = res {
        return Err(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err)));
    }
    let res = res.unwrap();

    // profile name in proxy server
    let name = format!("{}_{}", src_backend, &profile.name);

    // if the profile is already in the database, just return it
    // before return, update profile name record
    if let Some(row) = res {
        let mut active_model: crate::entity::profiles::ActiveModel = row.clone().into();
        active_model.name = ActiveValue::Set(name.clone());
        active_model.src_name = ActiveValue::Set(profile.name.clone());
        if let Err(err) = save_profile(active_model).await {
            return Err(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err)));
        }
        return Ok(Profile {
            id: if is_need_to_trans { row.uuid } else { profile.id },
            name: if is_need_to_trans { name } else { profile.name },
            properties: re_signature(src_backend, profile.properties).await,
        });
    }

    // no record, to create one, and assign the proxy server UUID for it

    // // check whether backend server UUID can be used directly or not,
    // // as no the same UUID in proxy server database so far.
    // let check = find_by_uuid(&profile.id).await;
    // if let Err(err) = check {
    //     return Err(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err)));
    // }
    // let check = check.unwrap();
    // let uuid = match check {
    //     None => { profile.id.clone() } // use backend server UUID directly
    //     Some(_) => { Uuid::new_v4().to_string().replace("-", "") } // generate a new UUID
    // };

    // no matter what type of creation it is, just using Version 4 UUID.
    // if src_backend is a main server, then return its src_uuid.
    // and if it doesn't enable the main server prefix, return src_name as well.
    let uuid = Uuid::new_v4().to_string().replace("-", "");

    // insert a new profile record into database
    let record = crate::entity::profiles::ActiveModel {
        id: ActiveValue::NotSet,
        backend_id: ActiveValue::Set(src_backend.to_string()),
        src_name: ActiveValue::Set(profile.name.clone()),
        src_uuid: ActiveValue::Set(profile.id.clone()),
        uuid: ActiveValue::Set(uuid.clone()),
        name: ActiveValue::Set(name.clone()),
    };
    if let Err(err) = save_profile(record).await {
        return Err(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err)));
    }

    Ok(Profile {
        id: if is_need_to_trans { uuid } else { profile.id },
        name: if is_need_to_trans { name } else { profile.name },
        properties: re_signature(src_backend, profile.properties).await,
    })
}