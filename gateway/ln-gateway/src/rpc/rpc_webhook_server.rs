use std::net::SocketAddr;
use std::sync::mpsc;

use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Extension, Json, Router};
use axum_macros::debug_handler;
use bitcoin_hashes::hex::ToHex;
use fedimint_core::task::TaskGroup;
use fedimint_ln_client::pay::PayInvoicePayload;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::json;
use tower_http::cors::CorsLayer;
use tower_http::validate_request::ValidateRequestHeaderLayer;
use tracing::{error, instrument};

use super::{
    BackupPayload, BalancePayload, ConnectFedPayload, DepositAddressPayload, LeaveFedPayload,
    RestorePayload, SetConfigurationPayload, WithdrawPayload,
};
use crate::db::GatewayConfiguration;
use crate::gateway_lnrpc::InterceptHtlcRequest;
use crate::rpc::ConfigPayload;
use crate::{Gateway, GatewayError};

pub async fn run_webhook_server(
    bind_addr: SocketAddr,
    task_group: &mut TaskGroup,
    htlc_stream_sender: mpsc::Sender<Result<InterceptHtlcRequest, tonic::Status>>,
) -> axum::response::Result<()> {
    let app = Router::new()
            .route("/webhook", post(handle_htlc))
            .layer(Extension(htlc_stream_sender.clone()));

    let handle = task_group.make_handle();
    let shutdown_rx = handle.make_shutdown_rx().await;
    let server = axum::Server::bind(&bind_addr).serve(app.into_make_service());
    task_group
        .spawn("Gateway Webhook Server", move |_| async move {
            let graceful = server.with_graceful_shutdown(async {
                shutdown_rx.await;
            });

            if let Err(e) = graceful.await {
                error!("Error shutting down gatewayd webhook server: {:?}", e);
            }
        })
        .await;

    Ok(())
}


/// `WebhookHandleHtlcParams` is a structure that holds an intercepted HTLC request.
/// 
/// Example JSON representation:
/// ```json
/// {
///     "htlc": {
///         "payment_hash": "a3f1e3b56a...",
///         "incoming_amount_msat": 1000,
///         "outgoing_amount_msat": 900,
///         "incoming_expiry": 300,
///         "short_channel_id": 2, // This is the short channel id of the federation mapping
///         "incoming_chan_id": 987654321,
///         "htlc_id": 12345
///     }
/// }
/// ```
struct WebhookHandleHtlcParams {
    htlc: InterceptHtlcRequest,
}

use serde::de::{MapAccess, Visitor};
use std::fmt;

impl<'de> Deserialize<'de> for WebhookHandleHtlcParams {
    fn deserialize<D>(deserializer: D) -> Result<WebhookHandleHtlcParams, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct WebhookHandleHtlcParamsVisitor;

        impl<'de> Visitor<'de> for WebhookHandleHtlcParamsVisitor {
            type Value = WebhookHandleHtlcParams;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct WebhookHandleHtlcParams")
            }

            fn visit_map<A>(self, mut map: A) -> Result<WebhookHandleHtlcParams, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut htlc = InterceptHtlcRequest::default();

                while let Some(key) = map.next_key()? {
                    match key {
                        "payment_hash" => htlc.payment_hash = map.next_value()?,
                        "incoming_amount_msat" => htlc.incoming_amount_msat = map.next_value()?,
                        "outgoing_amount_msat" => htlc.outgoing_amount_msat = map.next_value()?,
                        "incoming_expiry" => htlc.incoming_expiry = map.next_value()?,
                        "short_channel_id" => htlc.short_channel_id = map.next_value()?,
                        "incoming_chan_id" => htlc.incoming_chan_id = map.next_value()?,
                        "htlc_id" => htlc.htlc_id = map.next_value()?,
                        _ => (),
                    }
                }

                Ok(WebhookHandleHtlcParams { htlc })
            }
        }

        deserializer.deserialize_struct("WebhookHandleHtlcParams", &[
            "payment_hash",
            "incoming_amount_msat",
            "outgoing_amount_msat",
            "incoming_expiry",
            "short_channel_id",
            "incoming_chan_id",
            "htlc_id",
        ], WebhookHandleHtlcParamsVisitor)
    }
}

#[derive(Serialize)]
struct WebhookHandleHtlcResponse {
    preimage: String,
}

#[debug_handler]
async fn handle_htlc(
    Extension(htlc_stream_sender): Extension<mpsc::Sender<Result<InterceptHtlcRequest, tonic::Status>>>,
    params: Json<WebhookHandleHtlcParams>,
) -> Result<Json<WebhookHandleHtlcResponse>, GatewayError> {
    htlc_stream_sender.send(Ok(params.htlc)).map_err(|e| {
        error!("Error sending htlc to stream: {:?}", e);
        anyhow::anyhow!("Error sending htlc to stream: {:?}", e)
    })?;

    let (sender, receiver) = tokio::sync::oneshot::channel();

    tokio::spawn(async move {
        let res = receiver.await;
    });

    Ok(Json(WebhookHandleHtlcResponse {
        preimage: "preimage".to_string(),
    }))
}
