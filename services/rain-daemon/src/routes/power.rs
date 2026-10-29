use crate::response::{json_response, HttpResponse};
use crate::state::AppState;
use hyper::body::Incoming;
use hyper::{Method, Request, StatusCode};
use rain_protocol::ApiResponse;
use tracing::info;

pub(super) async fn handle(
    req: Request<Incoming>,
    state: &AppState,
) -> Result<HttpResponse, hyper::Error> {
    let method = req.method().clone();
    let path = req.uri().path().to_string();

    match (method, path.as_str()) {
        // --- TEST & POWER SIMULATION CONTROL ---
        (Method::POST, "/api/v1/power/simulate") => {
            std::env::set_var("RAIN_SIMULATE_BATTERY", "5");
            info!("Test battery mode activated: RAIN_SIMULATE_BATTERY=5 (Critical Discharging ⚠️)");
            Ok(json_response(StatusCode::OK, &ApiResponse::ok("Simulated battery set to 5% (Critical Discharging). Refresh Machines view to inspect.")))
        }

        (Method::POST, "/api/v1/power/reset") => {
            std::env::remove_var("RAIN_SIMULATE_BATTERY");
            info!("Test battery mode deactivated. Reading live kernel sysfs power sensors.");
            Ok(json_response(
                StatusCode::OK,
                &ApiResponse::ok("Battery mode restored to live hardware sensors."),
            ))
        }

        // --- AUTONOMOUS BATTERY & POWER FAILOVER ---
        (Method::POST, "/api/v1/power/failover") => {
            let convs = state.chat.list_conversations().await;
            let conv_id = if let Some(first_conv) = convs.first() {
                first_conv.id
            } else {
                let auto_conv = state
                    .chat
                    .create_conversation(
                        "System Failover & Evacuation".to_string(),
                        true,
                        vec!["human-user".to_string()],
                    )
                    .await;
                auto_conv.id
            };
            state
                .chat
                .execute_battery_failover_flow(conv_id, "Emergency battery evacuation".to_string())
                .await;
            Ok(json_response(
                StatusCode::OK,
                &ApiResponse::ok("Autonomous battery failover executed."),
            ))
        }

        _ => Ok(json_response(
            StatusCode::NOT_FOUND,
            &ApiResponse::<()>::err(format!("Route {} not found", path)),
        )),
    }
}
