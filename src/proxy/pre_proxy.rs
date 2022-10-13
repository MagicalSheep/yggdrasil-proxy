use std::collections::HashMap;
use futures::future::join_all;
use warp::hyper::StatusCode;
use crate::model::{AccessClaims, Profile, Property};
use crate::model::errors::CustomError;
use crate::model::request::{JoinQuery, JoinRequest, ProfileQuery, RefreshRequest, ValidateRequest};
use crate::proxy::decode_token;
use crate::repository::{find_by_name, find_by_uuid};

pub async fn refresh_pre_proxy(request: RefreshRequest) -> Result<(String, AccessClaims, RefreshRequest), CustomError> {
    // check token
    let access_claims = match decode_token(&request.access_token) {
        Ok(val) => { val }
        Err(err) => { return Err(err); }
    };

    let mut properties: Option<Vec<Property>> = None; // raw properties

    // check selected_profile field
    let selected_uuid = match request.selected_profile.as_ref() {
        None => {
            // selected_uuid should not be none
            if None == access_claims.selected_uuid {
                return Err(CustomError::ForbiddenOperationException(StatusCode::FORBIDDEN, "Invalid token.".to_string()));
            }
            access_claims.selected_uuid.as_ref().unwrap().clone()
        }
        Some(profile) => {
            // selected_uuid should be none
            if None != access_claims.selected_uuid {
                return Err(CustomError::IllegalArgumentException(StatusCode::BAD_REQUEST, "Access token already has a profile assigned.".to_string()));
            }
            properties = profile.properties.clone();
            profile.id.clone()
        }
    };

    let dst; // destination backend server id, it always has an valid value

    match access_claims.uuids.get(&selected_uuid) {
        None => {
            // happen when receiving an invalid profile uuid
            return Err(CustomError::IllegalArgumentException(StatusCode::BAD_REQUEST, "Invalid uuid.".to_string()));
        }
        Some(id) => { dst = id.clone() }
    }

    // build request to backend server
    let src_uuid;
    let src_name;
    match find_by_uuid(&selected_uuid).await {
        Ok(res) => {
            match res {
                None => {
                    return Err(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, "No such profile".to_string()));
                }
                Some(row) => {
                    src_uuid = row.src_uuid;
                    src_name = row.src_name;
                }
            }
        }
        Err(err) => {
            return Err(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err)));
        }
    }
    let profile = Profile {
        id: src_uuid,
        name: src_name,
        properties,
    };
    let access_token = access_claims.tokens.get(&dst).unwrap().clone();
    let is_selected = access_claims.selected.get(&dst).unwrap().clone();
    Ok((dst, access_claims, RefreshRequest {
        access_token,
        client_token: request.client_token,
        request_user: request.request_user,
        selected_profile: if let None = request.selected_profile { None } else if is_selected { None } else { Some(profile) },
    }))
}

pub async fn join_pre_proxy(request: JoinRequest) -> Result<(String, JoinRequest), CustomError> {
    let access_claim;
    match decode_token(&request.access_token) {
        Ok(claim) => { access_claim = claim }
        Err(err) => { return Err(err); }
    }
    if let Some(selected_uuid) = access_claim.selected_uuid {
        if selected_uuid.ne(&request.selected_profile) {
            return Err(CustomError::ForbiddenOperationException(StatusCode::FORBIDDEN, "Invalid token.".to_string()));
        }
    } else {
        return Err(CustomError::ForbiddenOperationException(StatusCode::FORBIDDEN, "Invalid token.".to_string()));
    }

    let dst = access_claim.uuids.get(&request.selected_profile).unwrap();
    let access_token = access_claim.tokens.get(dst).unwrap().clone();
    let uuid = match find_by_uuid(&request.selected_profile).await {
        Ok(res) => {
            match res {
                None => { return Err(CustomError::HttpException(StatusCode::BAD_REQUEST, "profile is not valid".to_string())); }
                Some(row) => { row.src_uuid }
            }
        }
        Err(err) => { return Err(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, err.to_string())); }
    };

    let ret = JoinRequest {
        access_token,
        selected_profile: uuid,
        server_id: request.server_id,
    };
    Ok((dst.to_string(), ret))
}

pub async fn has_join_pre_proxy(query: JoinQuery) -> Result<(String, Vec<(String, String)>), CustomError> {
    let (dst, src_name) = match find_by_name(&query.username).await {
        Ok(res) => {
            match res {
                None => { return Err(CustomError::IllegalArgumentException(StatusCode::BAD_REQUEST, "no such profile".to_string())); }
                Some(row) => { (row.backend_id, row.src_name) }
            }
        }
        Err(err) => { return Err(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err))); }
    };
    let mut queries = vec![("username".to_string(), src_name), ("serverId".to_string(), query.server_id)];
    if let Some(ip) = query.ip {
        queries.push(("ip".to_string(), ip))
    };
    Ok((dst, queries))
}

pub async fn profile_pre_proxy(uuid: String, query: ProfileQuery) -> Result<(String, String, Vec<(String, String)>), CustomError> {
    let (dst, uuid) = match find_by_uuid(&uuid).await {
        Ok(res) => {
            match res {
                None => { return Err(CustomError::IllegalArgumentException(StatusCode::BAD_REQUEST, "No such profile".to_string())); }
                Some(row) => { (row.backend_id, row.src_uuid) }
            }
        }
        Err(err) => { return Err(CustomError::HttpException(StatusCode::INTERNAL_SERVER_ERROR, format!("{}", err))); }
    };
    let mut queries = vec![];
    if let Some(unsigned) = query.unsigned { queries.push(("unsigned".to_string(), unsigned.to_string())) }
    Ok((dst, uuid, queries))
}

pub async fn profiles_pre_proxy(request: Vec<String>) -> Result<HashMap<String, Vec<String>>, CustomError> {
    let mut futures = vec![];
    let mut ret = HashMap::new();
    for name in request {
        futures.push(tokio::spawn(async move {
            find_by_name(&name).await
        }));
    };
    let results = join_all(futures).await;
    let mut c_results = vec![];
    for res in results {
        if let Ok(r) = res {
            if let Ok(row) = r { c_results.push(row) }
        }
    }
    for res in c_results {
        if let Some(row) = res {
            if !ret.contains_key(&row.backend_id) {
                ret.insert(row.backend_id.clone(), vec![]);
            }
            let v = ret.get_mut(&row.backend_id).unwrap();
            v.push(row.src_name);
        }
    }
    Ok(ret)
}

pub async fn validate_pre_proxy(request: ValidateRequest) -> Result<HashMap<String, ValidateRequest>, CustomError> {
    let claims = match decode_token(&request.access_token) {
        Ok(claims) => { claims }
        Err(err) => { return Err(err); }
    };
    let mut ret = HashMap::new();
    for (dst, token) in claims.tokens {
        let request = ValidateRequest {
            access_token: token,
            client_token: None,
        };
        ret.insert(dst, request);
    };
    Ok(ret)
}