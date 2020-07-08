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
use std::sync::{Arc, Mutex};
use url::form_urlencoded;

use actix_web::http::HeaderMap;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer};

pub mod config;
pub mod errors;

lazy_static! {
    static ref MANDATORY_PARAMS: HashSet<String> = vec!["type".to_string(), "version".to_string()]
        .into_iter()
        .collect();
}

fn main() -> Result<(), Error> {
    let sys = actix::System::new("graph-breaker");

    let settings = config::AppSettings::assemble().context("could not assemble AppSettings")?;
    let service_addr = (settings.service.address, settings.service.port);
    let data = Arc::new(Mutex::new(settings));
    HttpServer::new(move || {
        App::new()
            .wrap(middleware::Logger::default())
            .data(data.clone())
            .route("/action", web::get().to(action))
    })
    .bind(service_addr)
    .context("failed to start server")?
    .run();

    let _ = sys.run();

    Ok(())
}

/// Make sure `query` string contains all `params` keys.
pub fn ensure_query_params(
    required_params: &HashSet<String>,
    query: &str,
) -> Result<(), errors::AppError> {
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
        return Err(errors::AppError::MissingParams(missing));
    }

    Ok(())
}

/// Check "Authorization" header has expected token
pub fn ensure_valid_bearer(headers: &HeaderMap, expected: &str) -> Result<(), errors::AppError> {
    let value = match headers.get(actix_web::http::header::AUTHORIZATION) {
        Some(v) => match v.to_str() {
            Ok(v) => v,
            Err(_) => return Err(errors::AppError::InvalidAuthenticationToken()),
        },
        None => return Err(errors::AppError::InvalidAuthenticationToken()),
    };

    let actual: Vec<&str> = value.split(' ').collect();
    if actual != vec!["Bearer", expected] {
        return Err(errors::AppError::InvalidAuthenticationToken());
    }

    Ok(())
}

async fn action(
    req: HttpRequest,
    app_data: web::Data<Arc<Mutex<config::AppSettings>>>,
) -> Result<HttpResponse, errors::AppError> {
    // Throw error on invalid params
    ensure_query_params(&MANDATORY_PARAMS, req.query_string())?;

    // Ensure request has valid bearer token
    let expected_token = app_data.lock().unwrap().service.client_auth_token.clone();
    ensure_valid_bearer(req.headers(), &expected_token)?;

    let resp = HttpResponse::Ok()
        .content_type("application/json")
        .body("{}");
    Ok(resp)
}
