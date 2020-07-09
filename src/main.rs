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
extern crate anyhow;
#[macro_use]
extern crate serde_derive;
#[macro_use]
extern crate serde_json;
extern crate log;
extern crate serde_yaml;

use anyhow::{Context, Error};
use lazy_static::lazy_static;
use std::collections::{HashMap, HashSet};
use std::iter::Iterator;
use std::sync::{Arc, Mutex};
use url::form_urlencoded;

use actix_web::http::HeaderMap;
use actix_web::{middleware, web, App, HttpRequest, HttpResponse, HttpServer};

pub mod action;
pub mod config;
pub mod errors;
pub mod github;
pub mod graph_schema;

const TYPE_PARAM: &str = "type";
const VERSION_PARAM: &str = "version";

lazy_static! {
    static ref MANDATORY_PARAMS: HashSet<String> =
        vec![TYPE_PARAM.to_string(), VERSION_PARAM.to_string()]
            .into_iter()
            .collect();
}

fn main() -> Result<(), Error> {
    let settings = config::AppSettings::assemble().context("could not assemble AppSettings")?;
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::Builder::from_default_env()
        .filter(Some(module_path!()), settings.verbosity)
        .init();

    let sys = actix::System::new("graph-breaker");

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
) -> Result<HashMap<String, String>, errors::AppError> {
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

    // Return a k-v hashmap
    Ok(form_urlencoded::parse(query.as_bytes())
        .into_owned()
        .collect())
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
    let settings = app_data.lock().unwrap();

    // Throw error on invalid params
    let kv_hashmap = ensure_query_params(&MANDATORY_PARAMS, req.query_string())?;

    // Ensure request has valid bearer token
    let expected_token = settings.service.client_auth_token.clone();
    ensure_valid_bearer(req.headers(), &expected_token)?;

    // Validate action
    let action_type = action::ensure_valid_action_type(kv_hashmap.get(TYPE_PARAM).unwrap())?;
    let version = kv_hashmap.get(VERSION_PARAM).unwrap();

    // Validate version
    let result = action::perform_action(action_type, version.clone(), settings.github.clone())
        .await
        .map_err(|msg| errors::AppError::ActionFailed(msg.to_string()))?;
    Ok(HttpResponse::from(result))
}
