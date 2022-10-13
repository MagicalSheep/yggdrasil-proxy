pub mod filters;
pub mod handlers;
mod api;

#[macro_export]
macro_rules! reject {
    ($err:expr) => {
        Err(warp::reject::custom($err))
    }
}