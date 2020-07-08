//! Available service actions

use crate::errors;

pub enum Action {
  Enable,
  Disable,
}

/// Check "Authorization" header has expected token
pub fn ensure_valid_action(value: &str) -> Result<Action, errors::AppError> {
  match value {
    "enable" => return Ok(Action::Enable),
    "disable" => return Ok(Action::Disable),
    _ => return Err(errors::AppError::InvalidAuthenticationToken()),
  }
}
