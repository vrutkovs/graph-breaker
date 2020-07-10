use actix_web::{http, HttpResponse};
use thiserror::Error;

#[derive(Debug, Error, Eq, PartialEq)]
/// Application-level errors
pub enum AppError {
  /// Invalid authentication token
  #[error("invalid authentication token")]
  InvalidAuthenticationToken(),

  /// Invalid github token
  #[error("invalid github token")]
  InvalidGithubToken(),

  /// Invalid action
  #[error("invalid action")]
  InvalidAction(String),

  /// Error performing action
  #[error("action failed")]
  ActionFailed(String),
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
      AppError::InvalidAuthenticationToken() => http::StatusCode::UNAUTHORIZED,
      AppError::InvalidAction(_) => http::StatusCode::BAD_REQUEST,
      AppError::InvalidGithubToken() => http::StatusCode::INTERNAL_SERVER_ERROR,
      AppError::ActionFailed(_) => http::StatusCode::INTERNAL_SERVER_ERROR,
    }
  }

  /// Return the kind for the error.
  pub fn kind(&self) -> String {
    let kind = match *self {
      AppError::InvalidAuthenticationToken() => "invalid_auth_token",
      AppError::InvalidAction(_) => "invalid_action",
      AppError::InvalidGithubToken() => "invalid_github_token",
      AppError::ActionFailed(_) => "action_failed",
    };
    kind.to_string()
  }

  /// Return the value for the error.
  pub fn value(&self) -> String {
    let error_msg = format!("{}", self);
    match self {
      AppError::InvalidAction(msg) | AppError::ActionFailed(msg) => {
        format!("{}: {}", error_msg, msg)
      }
      _ => error_msg,
    }
  }
}

impl actix_web::error::ResponseError for AppError {
  fn error_response(&self) -> HttpResponse {
    self.as_json_error()
  }
}
