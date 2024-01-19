use std::collections::BTreeMap;
use std::sync::Arc;

use tokio::sync::oneshot::Sender;
use tokio::sync::Mutex;

use self::rpc_client::LightningRpcError;
use crate::gateway_lnrpc::InterceptHtlcResponse;

pub mod lnd;
pub mod rpc_client;

pub async fn send_htlc_to_webhook(
    outcomes: &Arc<Mutex<BTreeMap<u64, Sender<InterceptHtlcResponse>>>>,
    htlc: InterceptHtlcResponse,
) -> Result<(), LightningRpcError> {
    let htlc_id = htlc.htlc_id;
    if let Some(sender) = outcomes.lock().await.remove(&htlc_id) {
        sender
            .send(htlc)
            .map_err(|_| LightningRpcError::FailedToCompleteHtlc {
                failure_reason: "Failed to send back to webhook".to_string(),
            })?;
        Ok(())
    } else {
        Err(LightningRpcError::FailedToCompleteHtlc {
            failure_reason: format!("Could not find sender for HTLC {}", htlc_id),
        })
    }
}
