use warp::http::StatusCode;

#[derive(Debug)]
pub enum CustomError {
    ForbiddenOperationException(StatusCode, String),
    IllegalArgumentException(StatusCode, String),
    HttpException(StatusCode, String),
}

impl warp::reject::Reject for CustomError {}

impl Into<String> for &CustomError {
    fn into(self) -> String {
        match self {
            CustomError::ForbiddenOperationException(_, _) => { "ForbiddenOperationException".to_string() }
            CustomError::IllegalArgumentException(_, _) => { "IllegalArgumentException".to_string() }
            CustomError::HttpException(_, msg) => { msg.clone() }
        }
    }
}