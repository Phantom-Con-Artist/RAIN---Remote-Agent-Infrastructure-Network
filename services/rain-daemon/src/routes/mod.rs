mod auth;
mod chat;
mod hub;
mod jobs;
mod power;
mod remote;
mod system;

use crate::portal::render_hub_portal_html;
use crate::response::{cors_preflight, html_response, static_response, HttpResponse};
use crate::state::AppState;
use hyper::body::Incoming;
use hyper::{Method, Request, StatusCode};

fn get_query_param(query: Option<&str>, key: &str) -> Option<String> {
    query?.split('&').find_map(|pair| {
        let mut parts = pair.splitn(2, '=');
        if parts.next()? == key {
            let val = parts.next()?;
            let decoded = val
                .replace("%20", " ")
                .replace("%2F", "/")
                .replace("%7E", "~");
            Some(decoded)
        } else {
            None
        }
    })
}

pub(crate) async fn handle_request(
    req: Request<Incoming>,
    state: AppState,
) -> Result<HttpResponse, hyper::Error> {
    let mutating = req.method() != Method::GET
        && req.method() != Method::HEAD
        && req.method() != Method::OPTIONS;
    let response = dispatch(req, state.clone()).await?;
    if mutating {
        if let Err(e) = state.chat.persist(&state.history_path).await {
            return Ok(crate::response::json_response(
                StatusCode::INTERNAL_SERVER_ERROR,
                &rain_protocol::ApiResponse::<()>::err(format!("History write failed: {e}")),
            ));
        }
    }
    Ok(response)
}

async fn dispatch(req: Request<Incoming>, state: AppState) -> Result<HttpResponse, hyper::Error> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    if method == Method::OPTIONS {
        return Ok(cors_preflight());
    }

    // 1. Root / UI / Static Assets
    if (method == Method::GET || method == Method::HEAD) && (path == "/" || path == "/index.html") {
        let dist_index = std::path::Path::new("apps/rain-client/dist/index.html");
        if dist_index.exists() {
            if let Ok(content) = std::fs::read_to_string(dist_index) {
                return Ok(html_response(StatusCode::OK, content));
            }
        }
        return Ok(html_response(StatusCode::OK, render_hub_portal_html()));
    }

    if (method == Method::GET || method == Method::HEAD) && path.starts_with("/assets/") {
        let asset_path =
            std::path::Path::new("apps/rain-client/dist").join(path.trim_start_matches('/'));
        if asset_path.exists() {
            let mime = if path.ends_with(".js") {
                "application/javascript"
            } else if path.ends_with(".css") {
                "text/css"
            } else if path.ends_with(".svg") {
                "image/svg+xml"
            } else if path.ends_with(".json") {
                "application/json"
            } else {
                "application/octet-stream"
            };
            if let Ok(bytes) = std::fs::read(asset_path) {
                return Ok(static_response(StatusCode::OK, mime, bytes));
            }
        }
    }

    if path.starts_with("/api/v1/conversations") || path == "/api/v1/sessions" {
        chat::handle(req, &state).await
    } else if path.starts_with("/api/v1/auth/") {
        auth::handle(req, &state).await
    } else if path.starts_with("/api/v1/hub/") {
        hub::handle(req, &state).await
    } else if path.starts_with("/api/v1/power/") {
        power::handle(req, &state).await
    } else if path.starts_with("/api/v1/jobs") || path.starts_with("/api/v1/mesh/") {
        jobs::handle(req, &state).await
    } else if path == "/health"
        || path == "/api/v1/health"
        || path.starts_with("/api/v1/system/")
        || path == "/api/v1/processes"
        || path.starts_with("/api/v1/capabilities/")
        || path.starts_with("/api/v1/agents")
    {
        system::handle(req, &state).await
    } else {
        remote::handle(req, &state).await
    }
}
