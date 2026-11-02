use crate::response::{json_response, HttpResponse};
use crate::state::AppState;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::{Method, Request, StatusCode};
use rain_protocol::{
    ApiResponse, ConversationExportFormat, CreateConversationRequest, DirectChatRequest,
    SendMessageRequest, UpdateMessageMetaRequest,
};

pub(super) async fn handle(
    req: Request<Incoming>,
    state: &AppState,
) -> Result<HttpResponse, hyper::Error> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    match (method, path.as_str()) {
        // --- CONVERSATIONS & CHAT ---
        (Method::GET, "/api/v1/conversations") => {
            let convs = state.chat.list_conversations().await;
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(convs)))
        }

        (Method::GET, "/api/v1/conversations/search") => {
            let q = super::get_query_param(req.uri().query(), "q").unwrap_or_default();
            let sender = super::get_query_param(req.uri().query(), "sender");
            let limit = super::get_query_param(req.uri().query(), "limit")
                .and_then(|l| l.parse::<usize>().ok());
            let results = state
                .chat
                .search_messages(None, &q, sender.as_deref(), limit)
                .await;
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(results)))
        }

        (Method::GET, path_str)
            if path_str.starts_with("/api/v1/conversations/")
                && path_str.ends_with("/search") =>
        {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() == 6 {
                if let Ok(conv_id) = uuid::Uuid::parse_str(parts[4]) {
                    let q = super::get_query_param(req.uri().query(), "q").unwrap_or_default();
                    let sender = super::get_query_param(req.uri().query(), "sender");
                    let limit = super::get_query_param(req.uri().query(), "limit")
                        .and_then(|l| l.parse::<usize>().ok());
                    let results = state
                        .chat
                        .search_messages(Some(conv_id), &q, sender.as_deref(), limit)
                        .await;
                    Ok(json_response(StatusCode::OK, &ApiResponse::ok(results)))
                } else {
                    Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        &ApiResponse::<()>::err("Invalid conversation UUID"),
                    ))
                }
            } else {
                Ok(json_response(
                    StatusCode::NOT_FOUND,
                    &ApiResponse::<()>::err("Endpoint not found"),
                ))
            }
        }

        (Method::GET, "/api/v1/conversations/bookmarks") => {
            let results = state.chat.get_bookmarked_messages(None).await;
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(results)))
        }

        (Method::GET, path_str)
            if path_str.starts_with("/api/v1/conversations/")
                && path_str.ends_with("/pinned") =>
        {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() == 6 {
                if let Ok(conv_id) = uuid::Uuid::parse_str(parts[4]) {
                    let results = state.chat.get_pinned_messages(conv_id).await;
                    Ok(json_response(StatusCode::OK, &ApiResponse::ok(results)))
                } else {
                    Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        &ApiResponse::<()>::err("Invalid conversation UUID"),
                    ))
                }
            } else {
                Ok(json_response(
                    StatusCode::NOT_FOUND,
                    &ApiResponse::<()>::err("Endpoint not found"),
                ))
            }
        }

        (Method::GET, path_str)
            if path_str.starts_with("/api/v1/conversations/")
                && path_str.ends_with("/bookmarks") =>
        {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() == 6 {
                if let Ok(conv_id) = uuid::Uuid::parse_str(parts[4]) {
                    let results = state.chat.get_bookmarked_messages(Some(conv_id)).await;
                    Ok(json_response(StatusCode::OK, &ApiResponse::ok(results)))
                } else {
                    Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        &ApiResponse::<()>::err("Invalid conversation UUID"),
                    ))
                }
            } else {
                Ok(json_response(
                    StatusCode::NOT_FOUND,
                    &ApiResponse::<()>::err("Endpoint not found"),
                ))
            }
        }

        (Method::GET, path_str)
            if path_str.starts_with("/api/v1/conversations/")
                && path_str.ends_with("/stats") =>
        {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() == 6 {
                if let Ok(conv_id) = uuid::Uuid::parse_str(parts[4]) {
                    match state.chat.get_conversation_stats(conv_id).await {
                        Ok(stats) => Ok(json_response(StatusCode::OK, &ApiResponse::ok(stats))),
                        Err(e) => Ok(json_response(
                            StatusCode::NOT_FOUND,
                            &ApiResponse::<()>::err(e),
                        )),
                    }
                } else {
                    Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        &ApiResponse::<()>::err("Invalid conversation UUID"),
                    ))
                }
            } else {
                Ok(json_response(
                    StatusCode::NOT_FOUND,
                    &ApiResponse::<()>::err("Endpoint not found"),
                ))
            }
        }

        (Method::GET, path_str)
            if path_str.starts_with("/api/v1/conversations/")
                && path_str.ends_with("/export") =>
        {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() == 6 {
                if let Ok(conv_id) = uuid::Uuid::parse_str(parts[4]) {
                    let format_param = super::get_query_param(req.uri().query(), "format")
                        .unwrap_or_else(|| "markdown".into());
                    let raw = super::get_query_param(req.uri().query(), "raw")
                        .map(|v| v == "true" || v == "1")
                        .unwrap_or(false);
                    let format = match format_param.to_lowercase().as_str() {
                        "json" => ConversationExportFormat::Json,
                        "plaintext" | "txt" | "text" => ConversationExportFormat::PlainText,
                        _ => ConversationExportFormat::Markdown,
                    };
                    match state.chat.export_transcript(conv_id, format).await {
                        Ok(export) => {
                            if raw {
                                let (mime, ext) = match format {
                                    ConversationExportFormat::Json => ("application/json", "json"),
                                    ConversationExportFormat::PlainText => ("text/plain", "txt"),
                                    ConversationExportFormat::Markdown => ("text/markdown", "md"),
                                };
                                let filename = format!("rain-transcript-{}.{}", conv_id, ext);
                                let res = hyper::Response::builder()
                                    .status(StatusCode::OK)
                                    .header("Content-Type", mime)
                                    .header(
                                        "Content-Disposition",
                                        format!("attachment; filename=\"{}\"", filename),
                                    )
                                    .header("Cache-Control", "no-store")
                                    .header("Access-Control-Allow-Origin", "*")
                                    .body(crate::response::full_body(export.content))
                                    .unwrap();
                                Ok(res)
                            } else {
                                Ok(json_response(StatusCode::OK, &ApiResponse::ok(export)))
                            }
                        }
                        Err(e) => Ok(json_response(
                            StatusCode::NOT_FOUND,
                            &ApiResponse::<()>::err(e),
                        )),
                    }
                } else {
                    Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        &ApiResponse::<()>::err("Invalid conversation UUID"),
                    ))
                }
            } else {
                Ok(json_response(
                    StatusCode::NOT_FOUND,
                    &ApiResponse::<()>::err("Endpoint not found"),
                ))
            }
        }

        (Method::POST, "/api/v1/conversations/create") => {
            let body_bytes = req.into_body().collect().await?.to_bytes();
            match serde_json::from_slice::<CreateConversationRequest>(&body_bytes) {
                Ok(create_req) => {
                    let conv = state
                        .chat
                        .create_conversation(
                            create_req.title,
                            create_req.is_group,
                            create_req.participant_ids,
                        )
                        .await;
                    Ok(json_response(StatusCode::CREATED, &ApiResponse::ok(conv)))
                }
                Err(e) => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err(format!("Invalid create conversation JSON: {}", e)),
                )),
            }
        }

        (Method::POST, "/api/v1/conversations/direct") => {
            let body_bytes = req.into_body().collect().await?.to_bytes();
            match serde_json::from_slice::<DirectChatRequest>(&body_bytes) {
                Ok(direct_req) => {
                    let conv = state
                        .chat
                        .get_or_create_direct_chat(&direct_req.agent_id)
                        .await;
                    Ok(json_response(StatusCode::OK, &ApiResponse::ok(conv)))
                }
                Err(e) => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err(format!("Invalid direct chat JSON: {}", e)),
                )),
            }
        }

        (Method::DELETE, path_str) if path_str.starts_with("/api/v1/conversations/") => {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() == 5 {
                if let Ok(conv_id) = uuid::Uuid::parse_str(parts[4]) {
                    match state.chat.delete_conversation(conv_id).await {
                        Ok(_) => Ok(json_response(
                            StatusCode::OK,
                            &ApiResponse::ok("Conversation deleted successfully"),
                        )),
                        Err(e) => Ok(json_response(
                            StatusCode::NOT_FOUND,
                            &ApiResponse::<()>::err(e),
                        )),
                    }
                } else {
                    Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        &ApiResponse::<()>::err("Invalid conversation UUID"),
                    ))
                }
            } else {
                Ok(json_response(
                    StatusCode::NOT_FOUND,
                    &ApiResponse::<()>::err("Endpoint not found"),
                ))
            }
        }

        (Method::POST, path_str)
            if path_str.starts_with("/api/v1/conversations/") && path_str.ends_with("/stop") =>
        {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() == 6 {
                if let Ok(conv_id) = uuid::Uuid::parse_str(parts[4]) {
                    match state.chat.stop_conversation(conv_id).await {
                        Ok(conv) => Ok(json_response(StatusCode::OK, &ApiResponse::ok(conv))),
                        Err(e) => Ok(json_response(
                            StatusCode::NOT_FOUND,
                            &ApiResponse::<()>::err(e),
                        )),
                    }
                } else {
                    Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        &ApiResponse::<()>::err("Invalid conversation UUID"),
                    ))
                }
            } else {
                Ok(json_response(
                    StatusCode::NOT_FOUND,
                    &ApiResponse::<()>::err("Endpoint not found"),
                ))
            }
        }

        (Method::GET, path_str)
            if path_str.starts_with("/api/v1/conversations/")
                && path_str.ends_with("/messages") =>
        {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() == 6 {
                if let Ok(conv_id) = uuid::Uuid::parse_str(parts[4]) {
                    let msgs = state.chat.get_messages(conv_id).await;
                    Ok(json_response(StatusCode::OK, &ApiResponse::ok(msgs)))
                } else {
                    Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        &ApiResponse::<()>::err("Invalid conversation UUID"),
                    ))
                }
            } else {
                Ok(json_response(
                    StatusCode::NOT_FOUND,
                    &ApiResponse::<()>::err("Endpoint not found"),
                ))
            }
        }

        (Method::POST, path_str)
            if path_str.starts_with("/api/v1/conversations/")
                && path_str.ends_with("/messages") =>
        {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() == 6 {
                if let Ok(conv_id) = uuid::Uuid::parse_str(parts[4]) {
                    let body_bytes = req.into_body().collect().await?.to_bytes();
                    match serde_json::from_slice::<SendMessageRequest>(&body_bytes) {
                        Ok(send_req) => {
                            match state
                                .chat
                                .send_human_message(
                                    conv_id,
                                    send_req.text,
                                    send_req.target_session_id,
                                    send_req.attachments,
                                )
                                .await
                            {
                                Ok(msg) => {
                                    Ok(json_response(StatusCode::CREATED, &ApiResponse::ok(msg)))
                                }
                                Err(err) => Ok(json_response(
                                    StatusCode::BAD_REQUEST,
                                    &ApiResponse::<()>::err(err),
                                )),
                            }
                        }
                        Err(e) => Ok(json_response(
                            StatusCode::BAD_REQUEST,
                            &ApiResponse::<()>::err(format!("Invalid request body: {}", e)),
                        )),
                    }
                } else {
                    Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        &ApiResponse::<()>::err("Invalid conversation UUID"),
                    ))
                }
            } else {
                Ok(json_response(
                    StatusCode::NOT_FOUND,
                    &ApiResponse::<()>::err("Endpoint not found"),
                ))
            }
        }

        (Method::PATCH, path_str)
            if path_str.starts_with("/api/v1/conversations/")
                && path_str.contains("/messages/") =>
        {
            let parts: Vec<&str> = path_str.split('/').collect();
            if parts.len() == 7 {
                if let (Ok(conv_id), Ok(msg_id)) = (
                    uuid::Uuid::parse_str(parts[4]),
                    uuid::Uuid::parse_str(parts[6]),
                ) {
                    let body_bytes = req.into_body().collect().await?.to_bytes();
                    match serde_json::from_slice::<UpdateMessageMetaRequest>(&body_bytes) {
                        Ok(meta_req) => {
                            match state
                                .chat
                                .update_message_meta(
                                    conv_id,
                                    msg_id,
                                    meta_req.is_pinned,
                                    meta_req.is_bookmarked,
                                    meta_req.bookmark_note,
                                    meta_req.tags,
                                )
                                .await
                            {
                                Ok(updated_msg) => {
                                    Ok(json_response(StatusCode::OK, &ApiResponse::ok(updated_msg)))
                                }
                                Err(err) => Ok(json_response(
                                    StatusCode::BAD_REQUEST,
                                    &ApiResponse::<()>::err(err),
                                )),
                            }
                        }
                        Err(e) => Ok(json_response(
                            StatusCode::BAD_REQUEST,
                            &ApiResponse::<()>::err(format!("Invalid metadata request body: {}", e)),
                        )),
                    }
                } else {
                    Ok(json_response(
                        StatusCode::BAD_REQUEST,
                        &ApiResponse::<()>::err("Invalid conversation or message UUID"),
                    ))
                }
            } else {
                Ok(json_response(
                    StatusCode::NOT_FOUND,
                    &ApiResponse::<()>::err("Endpoint not found"),
                ))
            }
        }

        (Method::PATCH, path)
            if path.starts_with("/api/v1/conversations/") && path.ends_with("/collaboration") =>
        {
            let parts: Vec<_> = path.split('/').collect();
            let body = req.into_body().collect().await?.to_bytes();
            let result = match (
                parts.len(),
                parts.get(4).and_then(|v| uuid::Uuid::parse_str(v).ok()),
                serde_json::from_slice::<rain_protocol::CollaborationRequest>(&body),
            ) {
                (6, Some(id), Ok(settings)) => {
                    state.chat.configure_collaboration(id, settings).await
                }
                _ => Err("Invalid conversation or coworker settings".into()),
            };
            Ok(match result {
                Ok(c) => json_response(StatusCode::OK, &ApiResponse::ok(c)),
                Err(e) => json_response(StatusCode::BAD_REQUEST, &ApiResponse::<()>::err(e)),
            })
        }
        (Method::POST, path)
            if path.starts_with("/api/v1/conversations/") && path.ends_with("/resume") =>
        {
            let parts: Vec<_> = path.split('/').collect();
            let result = match (
                parts.len(),
                parts.get(4).and_then(|v| uuid::Uuid::parse_str(v).ok()),
            ) {
                (6, Some(id)) => state.chat.resume_conversation(id).await,
                _ => Err("Invalid conversation UUID".into()),
            };
            Ok(match result {
                Ok(c) => json_response(StatusCode::OK, &ApiResponse::ok(c)),
                Err(e) => json_response(StatusCode::BAD_REQUEST, &ApiResponse::<()>::err(e)),
            })
        }

        (Method::GET, path)
            if path.starts_with("/api/v1/sessions/") && path.ends_with("/groups") =>
        {
            let parts: Vec<_> = path.split('/').collect();
            match (
                parts.len(),
                parts.get(4).and_then(|id| uuid::Uuid::parse_str(id).ok()),
            ) {
                (6, Some(id)) => Ok(json_response(
                    StatusCode::OK,
                    &ApiResponse::ok(state.chat.group_context(id).await),
                )),
                _ => Ok(json_response(
                    StatusCode::BAD_REQUEST,
                    &ApiResponse::<()>::err("Invalid agent session UUID"),
                )),
            }
        }

        (Method::POST, path)
            if path.starts_with("/api/v1/conversations/") && path.ends_with("/native-terminal") =>
        {
            let id = path
                .split('/')
                .nth(4)
                .and_then(|id| uuid::Uuid::parse_str(id).ok());
            let result = match id {
                Some(id) => state.chat.native_terminal(id).await,
                None => Err("Invalid conversation UUID".into()),
            };
            Ok(match result {
                Ok(c) => json_response(StatusCode::OK, &ApiResponse::ok(c)),
                Err(e) => json_response(StatusCode::BAD_REQUEST, &ApiResponse::<()>::err(e)),
            })
        }
        (Method::GET, path)
            if path.starts_with("/api/v1/conversations/") && path.ends_with("/commands") =>
        {
            let id = path
                .split('/')
                .nth(4)
                .and_then(|id| uuid::Uuid::parse_str(id).ok());
            let result = match id {
                Some(id) => state.chat.command_catalog(id).await,
                None => Err("Invalid conversation UUID".into()),
            };
            Ok(match result {
                Ok(c) => json_response(StatusCode::OK, &ApiResponse::ok(c)),
                Err(e) => json_response(StatusCode::BAD_REQUEST, &ApiResponse::<()>::err(e)),
            })
        }

        (Method::PATCH, path)
            if path.starts_with("/api/v1/conversations/") && path.ends_with("/mode") =>
        {
            let id = path
                .split('/')
                .nth(4)
                .and_then(|v| uuid::Uuid::parse_str(v).ok());
            let body = req.into_body().collect().await?.to_bytes();
            let result = match (
                id,
                serde_json::from_slice::<rain_protocol::ModeRequest>(&body),
            ) {
                (Some(id), Ok(settings)) => state.chat.set_mode(id, settings).await,
                _ => Err("Invalid mode settings".into()),
            };
            Ok(match result {
                Ok(c) => json_response(StatusCode::OK, &ApiResponse::ok(c)),
                Err(e) => json_response(StatusCode::BAD_REQUEST, &ApiResponse::<()>::err(e)),
            })
        }
        (Method::GET, path)
            if path.starts_with("/api/v1/conversations/") && path.ends_with("/activity") =>
        {
            let result = match path
                .split('/')
                .nth(4)
                .and_then(|v| uuid::Uuid::parse_str(v).ok())
            {
                Some(id) => state.chat.conversation_activity(id).await,
                None => Err("Invalid conversation".into()),
            };
            Ok(match result {
                Ok(c) => json_response(StatusCode::OK, &ApiResponse::ok(c)),
                Err(e) => json_response(StatusCode::BAD_REQUEST, &ApiResponse::<()>::err(e)),
            })
        }
        (Method::POST, path)
            if path.starts_with("/api/v1/conversations/") && path.ends_with("/agent-control") =>
        {
            let id = path
                .split('/')
                .nth(4)
                .and_then(|v| uuid::Uuid::parse_str(v).ok());
            let body = req.into_body().collect().await?.to_bytes();
            let data = serde_json::from_slice::<serde_json::Value>(&body).unwrap_or_default();
            let agent = data["agent_id"]
                .as_str()
                .and_then(|v| uuid::Uuid::parse_str(v).ok());
            let action = data["action"].as_str().unwrap_or("");
            let result = match (id, agent, action) {
                (Some(id), Some(agent), "interrupt" | "terminate") => {
                    state
                        .chat
                        .control_member(id, agent, action == "terminate")
                        .await
                }
                _ => Err("Invalid agent control".into()),
            };
            Ok(match result {
                Ok(()) => json_response(StatusCode::OK, &ApiResponse::ok(())),
                Err(e) => json_response(StatusCode::BAD_REQUEST, &ApiResponse::<()>::err(e)),
            })
        }
        (Method::GET, "/api/v1/sessions") => {
            let sessions = state.chat.list_sessions().await;
            Ok(json_response(StatusCode::OK, &ApiResponse::ok(sessions)))
        }

        _ => Ok(json_response(
            StatusCode::NOT_FOUND,
            &ApiResponse::<()>::err(format!("Route {} not found", path)),
        )),
    }
}
