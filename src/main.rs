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

use actix_web::dev::ServiceRequest;
use actix_web::http::header::CONTENT_TYPE;
use actix_web::{guard, middleware, web, App, HttpResponse, HttpServer};
use actix_web_httpauth::extractors::bearer::BearerAuth;
use actix_web_httpauth::middleware::HttpAuthentication;

pub mod action;
pub mod config;
pub mod errors;
pub mod github;
pub mod graph_schema;

fn main() -> Result<(), Error> {
    let settings = config::AppSettings::assemble().context("could not assemble AppSettings")?;
    std::env::set_var("RUST_LOG", "actix_web=debug");
    env_logger::Builder::from_default_env()
        .filter(Some(module_path!()), settings.verbosity)
        .init();

    let sys = actix::System::new("graph-breaker");

    let service_addr = (settings.service.address, settings.service.port);
    let data = web::Data::new(settings);

    HttpServer::new(move || {
        let auth = HttpAuthentication::bearer(bearer_validator);
        App::new()
            .app_data(data.clone())
            .wrap(middleware::Logger::default())
            .wrap(auth)
            .data(web::JsonConfig::default().limit(4096))
            .service(
                web::resource("/action")
                    .guard(guard::Header(CONTENT_TYPE.as_str(), "application/json"))
                    .route(web::post().to(action)),
            )
    })
    .bind(service_addr)
    .context("failed to start server")?
    .run();

    let _ = sys.run();

    Ok(())
}

/// Check "Authorization" header has expected token
async fn bearer_validator(
    req: ServiceRequest,
    _credentials: BearerAuth,
) -> Result<ServiceRequest, actix_web::Error> {
    let settings = req.app_data::<config::AppSettings>().unwrap();
    if _credentials.token() == settings.service.client_auth_token {
        Ok(req)
    } else {
        Err(errors::AppError::InvalidAuthenticationToken().into())
    }
}

async fn action(
    settings: web::Data<config::AppSettings>,
    item: web::Json<action::Action>,
) -> Result<HttpResponse, errors::AppError> {
    // Perform action
    let result = action::perform_action(item.into_inner(), settings.github.clone())
        .await
        .map_err(|msg| errors::AppError::ActionFailed(msg.to_string()))?;
    Ok(HttpResponse::from(result))
}
