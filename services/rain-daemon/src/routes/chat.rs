use crate::response::{json_response, HttpResponse};
use crate::state::AppState;
use http_body_util::BodyExt;
use hyper::body::Incoming;
use hyper::{Method, Request, StatusCode};
use rain_protocol::{
    ApiResponse, CreateConversationRequest, DirectChatRequest, SendMessageRequest,
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
