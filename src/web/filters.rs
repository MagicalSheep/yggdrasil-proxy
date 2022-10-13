use warp::{Filter, Reply, Rejection};
use crate::handlers;
use crate::model::request::{JoinQuery, ProfileQuery};

/// POST /authserver/authenticate
pub fn authenticate() -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path!("authserver" / "authenticate")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(handlers::authenticate)
}

/// POST /authserver/refresh
pub fn fresh() -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path!("authserver" / "refresh")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(handlers::refresh)
}

/// POST /authserver/validate
pub fn validate() -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path!("authserver" / "validate")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(handlers::validate)
}

/// POST /authserver/invalidate
pub fn invalidate() -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path!("authserver" / "invalidate")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(handlers::invalidate)
}

/// POST /authserver/signout
pub fn logout() -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path!("authserver" / "signout")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(handlers::logout)
}

/// POST /sessionserver/session/minecraft/join
pub fn join() -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path!("sessionserver" / "session" / "minecraft" / "join")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(handlers::join)
}

/// GET /sessionserver/session/minecraft/hasJoined
pub fn has_join() -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path!("sessionserver" / "session" / "minecraft" / "hasJoined")
        .and(warp::get())
        .and(warp::query::<JoinQuery>())
        .and_then(handlers::has_join)
}

/// GET /sessionserver/session/minecraft/profile/{uuid}
pub fn profile() -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path!("sessionserver" / "session" / "minecraft" / "profile" / String)
        .and(warp::get())
        .and(warp::query::<ProfileQuery>())
        .and_then(handlers::profile)
}

/// POST /api/profiles/minecraft
pub fn profiles() -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path!("api" / "profiles" / "minecraft")
        .and(warp::post())
        .and(warp::body::json())
        .and_then(handlers::profiles)
}

/// GET /
pub fn meta() -> impl Filter<Extract=impl Reply, Error=Rejection> + Clone {
    warp::path::end()
        .and(warp::get())
        .and_then(handlers::meta)
}