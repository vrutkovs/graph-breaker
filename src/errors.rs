use actix_web::{http, HttpResponse};
use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
/// Application-level errors
pub enum AppError {
  /// Missing client parameters.
  #[error("mandatory client parameters missing")]
  MissingParams(Vec<String>),

  /// Invalid authentication token
  #[error("invalid authentication token")]
  InvalidAuthenticationToken(),
}

impl AppError {
  /// Return the HTTP JSON error response.
  pub fn as_json_error(&self) -> HttpResponse {
    let code = self.status_code();
    let json_body = json!({
        "kind": self.kind(),
        "value": self.value(),
    });
    HttpResponse::build(code).json(json_body)
  }

  /// Return the HTTP status code for the error.
  pub fn status_code(&self) -> http::StatusCode {
    match *self {
      AppError::MissingParams(_) => http::StatusCode::BAD_REQUEST,
      AppError::InvalidAuthenticationToken() => http::StatusCode::BAD_REQUEST,
    }
  }

  /// Return the kind for the error.
  pub fn kind(&self) -> String {
    let kind = match *self {
      AppError::MissingParams(_) => "missing_params",
      AppError::InvalidAuthenticationToken() => "invalid_auth_token",
    };
    kind.to_string()
  }

  /// Return the value for the error.
  pub fn value(&self) -> String {
    let error_msg = format!("{}", self);
    match self {
      AppError::MissingParams(params) => format!("{}: {}", error_msg, params.join(", ")),
      _ => error_msg,
    }
  }
}

impl actix_web::error::ResponseError for AppError {
  fn error_response(&self) -> HttpResponse {
    self.as_json_error()
  }
}
