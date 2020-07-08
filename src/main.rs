// Copyright 2020 Red Hat
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate serde_json;

use anyhow::{Context, Error};
use lazy_static::lazy_static;
use std::collections::HashSet;
use std::iter::Iterator;
use thiserror::Error;
use url::form_urlencoded;

use actix_web::{http, web, App, HttpRequest, HttpResponse, HttpServer};

pub mod config;

lazy_static! {
    static ref MANDATORY_PARAMS: HashSet<String> = vec!["type".to_string(), "version".to_string()]
        .into_iter()
        .collect();
}

// This struct represents state
#[derive(Clone)]
struct AppState {
    settings: config::AppSettings,
}

#[derive(Debug, Error, Eq, PartialEq)]
/// Application-level errors
pub enum AppError {
    /// Missing client parameters.
    #[error("mandatory client parameters missing")]
    MissingParams(Vec<String>),
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
        }
    }

    /// Return the kind for the error.
    pub fn kind(&self) -> String {
        let kind = match *self {
            AppError::MissingParams(_) => "missing_params",
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

fn main() -> Result<(), Error> {
    let sys = actix::System::new("graph-breaker");

    let settings = config::AppSettings::assemble().context("could not assemble AppSettings")?;
    let service_addr = (settings.service.address, settings.service.port);
    let state = AppState { settings };
    HttpServer::new(move || {
        App::new()
            .data(web::Data::new(state.clone()))
            .route("/action", web::get().to(action))
    })
    .bind(service_addr)
    .context("failed to start server")?
    .run();

    let _ = sys.run();

    Ok(())
}

/// Make sure `query` string contains all `params` keys.
pub fn ensure_query_params(required_params: &HashSet<String>, query: &str) -> Result<(), AppError> {
    // No mandatory parameters, always fine.
    if required_params.is_empty() {
        return Ok(());
    }

    // Extract and de-duplicate keys from input query.
    let query_keys: HashSet<String> = form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .map(|(k, _)| k)
        .collect();

    // Make sure no mandatory parameters are missing.
    let mut missing: Vec<String> = required_params.difference(&query_keys).cloned().collect();
    if !missing.is_empty() {
        missing.sort();
        return Err(AppError::MissingParams(missing));
    }

    Ok(())
}

async fn action(
    req: HttpRequest,
    _app_data: web::Data<AppState>,
) -> Result<HttpResponse, AppError> {
    ensure_query_params(&MANDATORY_PARAMS, req.query_string())?;

    let resp = HttpResponse::Ok()
        .content_type("application/json")
        .body("{}");
    Ok(resp)
}
