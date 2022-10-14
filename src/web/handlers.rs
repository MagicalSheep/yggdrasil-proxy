use std::collections::HashMap;
use std::convert::Infallible;
use std::error::Error;
use std::sync::{Arc};
use chrono::{Duration, Utc};
use futures::future::join_all;
use log::{debug, warn};
use reqwest::Client;
use warp::{Rejection, Reply};
use warp::http::StatusCode;
use crate::{CONFIG, Meta, reject};
use crate::model::{KeyPair, Profile};
use crate::model::errors::CustomError;
use crate::proxy::proxy::{authenticate_proxy, has_join_proxy, profile_proxy, profiles_proxy, refresh_proxy};
use crate::model::reply::{AuthenticateReply, CertificatesReply, ErrorReply, RefreshReply};
use crate::model::request::{AuthenticateRequest, JoinQuery, JoinRequest, LogoutRequest, ProfileQuery, RefreshRequest, ValidateRequest};
use crate::proxy::pre_proxy::{has_join_pre_proxy, validate_pre_proxy, join_pre_proxy, profile_pre_proxy, profiles_pre_proxy, refresh_pre_proxy};
use crate::utils::{decode_token, signature};
use crate::web::api::{AUTHENTICATE, HAS_JOIN, INVALIDATE, JOIN, PROFILE, PROFILES, REFRESH, SIGN_OUT, VALIDATE};

/// Send authenticate request to all backend servers, and ignore those unavailable replies.
///
/// After receiving all available access token and profiles information,
/// save them as jwt token. Pass jwt token as new access token to client side.
///
/// Of course, proxy will do the translating work for profile signature, uuid etc.
pub async fn authenticate(request: AuthenticateRequest) -> Result<impl Reply, Rejection> {
    let backends = &CONFIG.backends;

    let client = Arc::new(Client::new());
    let mut futures = vec![];

    for (id, url) in backends {
        let c_client = client.clone();
        let c_request = request.clone();
        futures.push(tokio::spawn(async move {
            let resp = match c_client.post(format!("{}{}", url, AUTHENTICATE)).json(&c_request).send().await {
                Ok(res) => { res }
                Err(err) => { return Err(err); }
            };
            match resp.json::<AuthenticateReply>().await {
                Ok(res) => { Ok((id.clone(), res)) }
                Err(err) => { Err(err) }
            }
        }));
    }

    let results = join_all(futures).await;
    let mut replies: HashMap<String, AuthenticateReply> = HashMap::new();

    for res in results {
        if let Err(_) = res { continue; }
        let i_res = res.unwrap();
        if let Err(_) = i_res { continue; }
        let (id, reply) = i_res.unwrap();
        debug!("Get authenticate reply from <{}>: {:#?}", &id, &reply);
        replies.insert(id, reply);
    }

    match authenticate_proxy(replies).await {
        Ok(reply) => { Ok(warp::reply::with_status(warp::reply::json(&reply), StatusCode::OK)) }
        Err(err) => { reject!(err) }
    }
}

pub async fn refresh(request: RefreshRequest) -> Result<impl Reply, Rejection> {
    debug!("Source request: {:#?}", request);
    let config = &*CONFIG;
    let res = refresh_pre_proxy(request).await;
    if let Err(err) = res { return Err(warp::reject::custom(err)); }
    let (dst, access_claims, req) = res.unwrap();
    debug!("Real request: {:#?}", req);
    let resp;
    // send request
    match config.backends.get(&dst) {
        None => {
            return reject!(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, "Invalid destination.".to_string()));
        }
        Some(url) => {
            match Client::new().post(format!("{}{}", url, REFRESH)).json(&req).send().await {
                Ok(res) => { resp = res }
                Err(err) => { return reject!(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err))); }
            }
        }
    }

    let resp = resp.text().await.unwrap();
    debug!("Source reply: {:#?}", resp);
    match serde_json::from_str::<RefreshReply>(&resp) {
        Ok(reply) => {
            match refresh_proxy(dst, access_claims, reply).await {
                Ok(reply) => {
                    debug!("Real reply: {:#?}", reply);
                    Ok(warp::reply::with_status(warp::reply::json(&reply), StatusCode::OK))
                }
                Err(err) => { reject!(err) }
            }
        }
        Err(_) => {
            match serde_json::from_str::<ErrorReply>(&resp) {
                Ok(reply) => {
                    debug!("Real reply: {:#?}", reply);
                    Ok(warp::reply::with_status(warp::reply::json(&reply), StatusCode::OK))
                }
                Err(err) => { reject!(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err))) }
            }
        }
    }
}

pub async fn validate(request: ValidateRequest) -> Result<impl Reply, Rejection> {
    let request = match validate_pre_proxy(request).await {
        Ok(res) => { res }
        Err(err) => { return reject!(err); }
    };
    let mut futures = vec![];
    for (dst, req) in request {
        futures.push(tokio::spawn(async move {
            let url = CONFIG.backends.get(&dst).unwrap();
            Client::new().post(format!("{}{}", url, VALIDATE)).json(&req).send().await
        }))
    }
    let results = join_all(futures).await;
    let mut ok = false;
    for res in results {
        if let Ok(r) = res {
            if let Ok(r) = r {
                if r.status() == StatusCode::NO_CONTENT { ok = true }
            }
        }
    }
    if ok {
        Ok(warp::reply::with_status(warp::reply::reply(), StatusCode::NO_CONTENT))
    } else {
        reject!(CustomError::ForbiddenOperationException(StatusCode::FORBIDDEN, "Invalid token.".to_string()))
    }
}

pub async fn invalidate(request: ValidateRequest) -> Result<impl Reply, Rejection> {
    let request = match validate_pre_proxy(request).await {
        Ok(res) => { res }
        Err(err) => { return reject!(err); }
    };
    for (dst, req) in request {
        tokio::spawn(async move {
            let url = CONFIG.backends.get(&dst).unwrap();
            Client::new().post(format!("{}{}", url, INVALIDATE)).json(&req).send().await
        });
    }
    Ok(warp::reply::with_status(warp::reply::reply(), StatusCode::NO_CONTENT))
}

/// Send sign out request to all backend servers and ignore replies
pub async fn logout(request: LogoutRequest) -> Result<impl Reply, Rejection> {
    let config = &*CONFIG;
    let backends = &config.backends;
    let client = Arc::new(Client::new());

    for url in backends.values() {
        let c_client = client.clone();
        let c_request = request.clone();
        tokio::spawn(async move {
            let _ = c_client.post(format!("{}{}", url, SIGN_OUT)).json(&c_request).send().await.map_err(|err| {
                warn!("{}", err)
            });
        });
    }
    Ok(warp::reply::with_status(warp::reply::reply(), StatusCode::NO_CONTENT))
}

pub async fn join(request: JoinRequest) -> Result<impl Reply, Rejection> {
    let config = &*CONFIG;
    let resp;
    match join_pre_proxy(request).await {
        Ok((dst, req)) => {
            let url = config.backends.get(&dst).unwrap();
            resp = Client::new().post(format!("{}{}", url, JOIN)).json(&req).send().await;
        }
        Err(err) => { return reject!(err); }
    }
    let resp = match resp {
        Ok(res) => { res }
        Err(err) => { return reject!(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())); }
    };

    let reply = resp.text().await.unwrap();

    match serde_json::from_str::<ErrorReply>(&reply) {
        Ok(rep) => { Ok(warp::reply::with_status(warp::reply::json(&rep), StatusCode::OK)) }
        Err(_) => { Ok(warp::reply::with_status(warp::reply::json(&reply), StatusCode::NO_CONTENT)) }
    }
}

pub async fn has_join(query: JoinQuery) -> Result<impl Reply, Rejection> {
    let config = &*CONFIG;
    let dst;
    let resp = match has_join_pre_proxy(query).await {
        Ok((d, queries)) => {
            dst = d;
            let url = config.backends.get(&dst).unwrap();
            Client::new().get(format!("{}{}", url, HAS_JOIN)).query(&queries).send().await
        }
        Err(err) => { return reject!(err); }
    };
    let resp = match resp {
        Ok(res) => { res }
        Err(err) => { return reject!(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())); }
    };
    let reply = resp.text().await.unwrap();

    match serde_json::from_str::<Profile>(&reply) {
        Ok(res) => {
            match has_join_proxy(&dst, res).await {
                Ok(ret) => { Ok(warp::reply::with_status(warp::reply::json(&ret), StatusCode::OK)) }
                Err(err) => { return reject!(err); }
            }
        }
        Err(_) => { Ok(warp::reply::with_status(warp::reply::json(&reply), StatusCode::NO_CONTENT)) }
    }
}

pub async fn profile(uuid: String, query: ProfileQuery) -> Result<impl Reply, Rejection> {
    let dst;
    let resp = match profile_pre_proxy(uuid, query).await {
        Ok((d, uuid, queries)) => {
            dst = d;
            match CONFIG.backends.get(&dst) {
                None => { return reject!(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, "Invalid backend server".to_string())); }
                Some(url) => {
                    Client::new().get(format!("{}{}{}", url, PROFILE, uuid)).query(&queries).send().await
                }
            }
        }
        Err(err) => {
            if let CustomError::IllegalArgumentException(_, _) = err {
                return Ok(warp::reply::with_status(warp::reply::json(&String::new()), StatusCode::NO_CONTENT));
            }
            return reject!(err);
        }
    };
    let resp = match resp {
        Ok(res) => { res }
        Err(err) => { return reject!(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())); }
    };

    if resp.status() == StatusCode::NO_CONTENT {
        Ok(warp::reply::with_status(warp::reply::json(&String::new()), StatusCode::NO_CONTENT))
    } else {
        match resp.json::<Profile>().await {
            Ok(profile) => {
                let ret = match profile_proxy(&dst, profile).await {
                    Ok(p) => { p }
                    Err(err) => { return reject!(err); }
                };
                Ok(warp::reply::with_status(warp::reply::json(&ret), StatusCode::OK))
            }
            Err(err) => { reject!(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())) }
        }
    }
}

pub async fn profiles(request: Vec<String>) -> Result<impl Reply, Rejection> {
    let p_request = match profiles_pre_proxy(request).await {
        Ok(v) => { v }
        Err(err) => { return reject!(err); }
    };
    let mut futures = vec![];
    for (dst, request) in p_request {
        futures.push(tokio::spawn(async move {
            let url = CONFIG.backends.get(&dst).unwrap();
            (dst, Client::new().post(format!("{}{}", url, PROFILES)).json(&request).send().await.unwrap())
        }));
    }
    let results = join_all(futures).await;
    let mut ret = HashMap::new();
    for res in results {
        if let Ok((dst, reply)) = res {
            if let Ok(profiles) = reply.json::<Vec<Profile>>().await {
                ret.insert(dst, profiles);
            }
        }
    }
    let ret = profiles_proxy(ret).await;
    Ok(warp::reply::with_status(warp::reply::json(&ret), StatusCode::OK))
}

pub async fn meta() -> Result<impl Reply, Rejection> {
    let config = &*CONFIG;
    Ok(warp::reply::with_status(warp::reply::json(&Meta::from(config)), StatusCode::OK))
}

/// Just create a random key pair.
/// This behaviour maybe changed if Minecraft updates the use of key pair in the future.
/// (Maybe implement the report system api)
pub async fn certificates(token: String) -> Result<impl Reply, Rejection> {
    match CONFIG.meta.enable_profile_key {
        Some(c) => {
            if !c {
                return reject!(CustomError::HttpException(StatusCode::NOT_FOUND, "enable_profile_key is not enabled".to_string()));
            }
        }
        None => {
            return reject!(CustomError::HttpException(StatusCode::NOT_FOUND, "enable_profile_key is not enabled".to_string()));
        }
    }
    // Work as Mojang
    if token.len() < 7 { return Ok(warp::reply::with_status(warp::reply::json(&String::new()), StatusCode::NO_CONTENT)); }
    if let Err(_) = decode_token(&token[7..token.len()]) {
        return Ok(warp::reply::with_status(warp::reply::json(&String::new()), StatusCode::NO_CONTENT));
    }

    let key_pair = match KeyPair::new() {
        Ok(res) => { res }
        Err(err) => { return reject!(err); }
    };
    let now = Utc::now();
    let expires_at = now + Duration::hours(48);
    let public_key_signature = signature(expires_at.timestamp_millis().to_string() + &key_pair.public_key);
    let ret = CertificatesReply {
        expires_at: expires_at.format("%+").to_string(),
        key_pair,
        public_key_signature_v2: public_key_signature.clone(),
        public_key_signature,
        refreshed_after: (now + Duration::hours(36)).format("%+").to_string(),
    };
    debug!("{:#?}", ret);
    Ok(warp::reply::with_status(warp::reply::json(&ret), StatusCode::OK))
}

pub async fn err_handle(err: Rejection) -> Result<impl Reply, Infallible> {
    let mut reply = ErrorReply {
        error: "Unknown Error".to_string(),
        error_message: "Unknown Error".to_string(),
        cause: None,
    };
    let code: StatusCode;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        reply.error = "404 Not Found".to_string();
        reply.error_message = "404 Not Found".to_string();
    } else if let Some(e) = err.find::<warp::filters::body::BodyDeserializeError>() {
        code = StatusCode::BAD_REQUEST;
        reply.error = "Bad Request".to_string();
        reply.error_message = match e.source() {
            Some(cause) => cause.to_string(),
            None => "Unknown error".to_string(),
        }
    } else if let Some(e) = err.find::<CustomError>() {
        let (c, err, err_msg) = match e {
            CustomError::ForbiddenOperationException(code, msg) => { (code.clone(), e.into(), msg.clone()) }
            CustomError::IllegalArgumentException(code, msg) => { (code.clone(), e.into(), msg.clone()) }
            CustomError::HttpException(code, msg) => { (code.clone(), e.into(), msg.clone()) }
        };
        code = c;
        reply.error = err;
        reply.error_message = err_msg;
    } else {
        code = StatusCode::INTERNAL_SERVER_ERROR;
        reply.error = "Interval Server Error".to_string();
        reply.error_message = "Interval server error".to_string()
    }

    Ok(warp::reply::with_status(warp::reply::json(&reply), code))
}