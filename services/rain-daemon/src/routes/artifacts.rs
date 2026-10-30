use super::get_query_param;
use crate::{
    artifact_body::{byte_range, ArtifactBody},
    response::{empty_body, json_response, HttpResponse},
    state::AppState,
};
use http_body_util::BodyExt;
use hyper::{body::Incoming, Method, Request, Response, StatusCode};
use rain_protocol::ApiResponse;
use tokio::io::AsyncWriteExt;
use uuid::Uuid;

pub(super) async fn handle(
    req: Request<Incoming>,
    state: &AppState,
) -> Result<HttpResponse, hyper::Error> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();
    if method == Method::POST && path == "/api/v1/artifacts/upload" {
        let name = get_query_param(req.uri().query(), "name").unwrap_or_else(|| "file".into());
        let name = std::path::Path::new(&name)
            .file_name()
            .and_then(|n| n.to_str())
            .filter(|n| !n.is_empty())
            .unwrap_or("file")
            .to_string();
        let folder = state
            .chat
            .artifacts
            .root()
            .join("uploads")
            .join(Uuid::new_v4().to_string());
        let result = async {
            tokio::fs::create_dir_all(&folder)
                .await
                .map_err(|e| e.to_string())?;
            let path = folder.join(name);
            let mut options = tokio::fs::OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            options.mode(0o600);
            let mut file = options.open(&path).await.map_err(|e| e.to_string())?;
            let mut body = req.into_body();
            while let Some(frame) = body.frame().await {
                let frame = frame.map_err(|e| e.to_string())?;
                if let Ok(data) = frame.into_data() {
                    file.write_all(&data).await.map_err(|e| e.to_string())?;
                }
            }
            file.sync_all().await.map_err(|e| e.to_string())?;
            drop(file);
            let artifact = state
                .chat
                .artifacts
                .capture(None, &path.to_string_lossy())
                .await?;
            let _ = tokio::fs::remove_file(path).await;
            Ok::<_, String>(artifact.attachment())
        }
        .await;
        return Ok(match result {
            Ok(a) => json_response(StatusCode::CREATED, &ApiResponse::ok(a)),
            Err(e) => json_response(StatusCode::BAD_REQUEST, &ApiResponse::<()>::err(e)),
        });
    }
    let parts: Vec<_> = path.split('/').collect();
    let id = parts.get(4).and_then(|id| Uuid::parse_str(id).ok());
    let Some(id) = id else {
        return Ok(json_response(
            StatusCode::BAD_REQUEST,
            &ApiResponse::<()>::err("Invalid shared file ID"),
        ));
    };
    let a = match state.chat.artifacts.get(id).await {
        Ok(a) => a,
        Err(e) => {
            return Ok(json_response(
                StatusCode::NOT_FOUND,
                &ApiResponse::<()>::err(e),
            ))
        }
    };
    if parts.len() == 5 && method == Method::GET {
        return Ok(json_response(StatusCode::OK, &ApiResponse::ok(a)));
    }
    if parts.len() != 6 || parts[5] != "content" || !matches!(method, Method::GET | Method::HEAD) {
        return Ok(json_response(
            StatusCode::NOT_FOUND,
            &ApiResponse::<()>::err("File endpoint not found"),
        ));
    }
    let etag = format!("\"{}\"", a.sha256);
    let range = req
        .headers()
        .get("range")
        .and_then(|h| h.to_str().ok())
        .filter(|_| {
            req.headers()
                .get("if-range")
                .and_then(|v| v.to_str().ok())
                .map(|v| v == etag)
                .unwrap_or(true)
        });
    let (start, length, status) = match range {
        Some(range) => match byte_range(range, a.size_bytes) {
            Ok((start, end)) => (start, end - start + 1, StatusCode::PARTIAL_CONTENT),
            Err(_) => {
                return Ok(Response::builder()
                    .status(StatusCode::RANGE_NOT_SATISFIABLE)
                    .header("content-range", format!("bytes */{}", a.size_bytes))
                    .header("access-control-allow-origin", "*")
                    .body(empty_body())
                    .unwrap())
            }
        },
        None => (0, a.size_bytes, StatusCode::OK),
    };
    let name = a
        .name
        .chars()
        .filter(|c| !c.is_control() && *c != '"' && *c != '\\')
        .collect::<String>();
    let disposition = if get_query_param(req.uri().query(), "download").as_deref() == Some("1")
        || a.mime_type == "text/html"
        || a.mime_type == "image/svg+xml"
    {
        "attachment"
    } else {
        "inline"
    };
    let mut builder = Response::builder()
        .status(status)
        .header("content-type", a.mime_type)
        .header("content-length", length.to_string())
        .header("accept-ranges", "bytes")
        .header("etag", etag)
        .header(
            "content-disposition",
            format!(
                "{disposition}; filename=\"{}\"",
                name.chars()
                    .map(|c| if c.is_ascii() { c } else { '_' })
                    .collect::<String>()
            ),
        )
        .header("x-content-type-options", "nosniff")
        .header("content-security-policy", "sandbox; default-src 'none'")
        .header("access-control-allow-origin", "*")
        .header(
            "access-control-expose-headers",
            "Content-Range, Content-Length, ETag, Accept-Ranges",
        );
    if status == StatusCode::PARTIAL_CONTENT {
        builder = builder.header(
            "content-range",
            format!("bytes {}-{}/{}", start, start + length - 1, a.size_bytes),
        );
    }
    Ok(builder
        .body(if method == Method::HEAD {
            empty_body()
        } else {
            ArtifactBody::new(state.chat.artifacts.clone(), id, start, length).boxed_unsync()
        })
        .unwrap())
}
