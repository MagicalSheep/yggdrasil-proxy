use std::collections::HashMap;
use chrono::{DateTime, Duration, Local};
use futures::future::join_all;
use reqwest::StatusCode;
use sea_orm::ActiveValue;
use crate::model::{AccessClaims, Profile, User};
use crate::model::errors::CustomError;
use crate::model::reply::{AuthenticateReply, RefreshReply};
use crate::proxy::{create_token, translate};
use crate::repository::{find_by_backend_and_uuid, save_profile};

pub async fn authenticate_proxy(reply: HashMap<String, AuthenticateReply>) -> Result<AuthenticateReply, CustomError> {
    if reply.is_empty() {
        return Err(CustomError::ForbiddenOperationException
            (StatusCode::FORBIDDEN, "Invalid credentials. Invalid username or password.".to_string()));
    }

    let mut profiles = vec![];
    let mut user: Option<User> = None; // it is not important
    let mut selected: HashMap<String, bool> = HashMap::new();
    let mut access_token: HashMap<String, String> = HashMap::new();
    let mut uuids: HashMap<String, String> = HashMap::new();
    let mut client_token = None;

    for (id, reply) in reply {
        for profile in reply.available_profiles {
            match translate(&id, profile).await {
                Ok(p) => {
                    uuids.insert(p.id.clone(), id.clone());
                    profiles.push(p);
                }
                Err(err) => return Err(err)
            }
        }
        // it is not important
        if let Some(u) = reply.user {
            if let None = user { user = Some(u) }
        }

        // never set selected profile in authenticate reply, because user may need to choose another backend server.
        // but we have to record each backend servers' selected status, so that if it can be known which
        // backend server access token is already bind to profile.
        match reply.selected_profile {
            None => { selected.insert(id.clone(), false); }
            Some(_) => { selected.insert(id.clone(), true); }
        }
        access_token.insert(id.clone(), reply.access_token);
        client_token = reply.client_token;
    }

    let now: DateTime<Local> = Local::now();
    let exp: DateTime<Local> = now + Duration::days(7);
    let access_claims = AccessClaims {
        tokens: access_token,
        uuids,
        selected,
        selected_uuid: None,
        exp: exp.timestamp_millis(),
    };

    Ok(AuthenticateReply {
        access_token: create_token(&access_claims),
        client_token,
        available_profiles: profiles,
        selected_profile: None,
        user,
    })
}

pub async fn refresh_proxy(backend: String, mut access_claims: AccessClaims, reply: RefreshReply) -> Result<RefreshReply, CustomError> {
    let selected_profile;
    if let Some(profile) = reply.selected_profile {
        // update profile src_name and name
        let row = match find_by_backend_and_uuid(&backend, &profile.id).await {
            Ok(res) => {
                match res {
                    None => { return Err(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, "No such profile".to_string())); }
                    Some(r) => { r }
                }
            }
            Err(err) => { return Err(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err))); }
        };
        let name = profile.name.clone();
        let model = crate::entity::profiles::ActiveModel {
            id: ActiveValue::Set(row.id),
            backend_id: ActiveValue::Set(row.backend_id),
            src_name: ActiveValue::Set(name.clone()),
            src_uuid: ActiveValue::Set(row.src_uuid),
            uuid: ActiveValue::Set(row.uuid),
            name: ActiveValue::Set(format!("{}_{}", &backend, name)),
        };
        if let Err(err) = save_profile(model).await {
            return Err(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err)));
        }

        match translate(&backend, profile).await {
            Ok(profile) => {
                access_claims.selected_uuid = Some(profile.id.clone());
                selected_profile = Some(profile)
            }
            Err(err) => { return Err(err); }
        }
        // this backend server token has been bind to a profile
        access_claims.selected.insert(backend.clone(), true);
    } else {
        access_claims.selected_uuid = None;
        // this backend server token does not bind to a profile
        access_claims.selected.insert(backend.clone(), false);
        selected_profile = None
    }

    access_claims.tokens.insert(backend, reply.access_token);
    let now: DateTime<Local> = Local::now();
    let exp: DateTime<Local> = now + Duration::days(7);
    access_claims.exp = exp.timestamp_millis();
    Ok(RefreshReply {
        access_token: create_token(&access_claims),
        client_token: reply.client_token,
        selected_profile,
        user: reply.user,
    })
}

pub async fn has_join_proxy(backend: &str, reply: Profile) -> Result<Profile, CustomError> {
    translate(backend, reply).await
}

pub async fn profile_proxy(backend: &str, reply: Profile) -> Result<Profile, CustomError> {
    translate(backend, reply).await
}

pub async fn profiles_proxy(reply: HashMap<String, Vec<Profile>>) -> Vec<Profile> {
    let mut futures = vec![];
    for (dst, profiles) in reply {
        futures.push(tokio::spawn(async move {
            let mut ps = vec![];
            for profile in profiles {
                ps.push(translate(&dst, profile).await.unwrap());
            }
            ps
        }));
    };
    let results = join_all(futures).await;
    let mut ret = vec![];
    for res in results {
        if let Ok(p) = res {
            ret.extend(p);
        }
    }
    ret
}