use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;
use std::sync::Arc;

use async_trait::async_trait;
use cln_rpc::primitives::ShortChannelId;
use fedimint_core::task::{sleep, TaskGroup};
use fedimint_core::Amount;
use fedimint_ln_common::route_hints::RouteHint;
use fedimint_ln_common::PrunedInvoice;
use secp256k1::PublicKey;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::{mpsc, Mutex};
use tokio::sync::oneshot::Sender;
use tonic::Status;
use tracing::info;

use crate::gateway_lnrpc::{
    EmptyResponse, GetNodeInfoResponse, GetRouteHintsResponse, InterceptHtlcRequest,
    InterceptHtlcResponse, PayInvoiceRequest, PayInvoiceResponse,
};
use crate::lnrpc_client::{
    ILnRpcClient, LightningRpcError, RouteHtlcStream, MAX_LIGHTNING_RETRIES,
};

type HtlcSubscriptionSender = mpsc::Sender<Result<InterceptHtlcRequest, Status>>;

const LND_PAYMENT_TIMEOUT_SECONDS: i32 = 180;

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AlbyPayResponse {
    amount: u64,
    description: String,
    destination: String,
    fee: u64,
    payment_hash: Vec<u8>,
    payment_preimage: Vec<u8>,
    payment_request: String,
}

pub struct GatewayAlbyClient {
    api_key: String,
    map: Arc<Mutex<BTreeMap<ShortChannelId, Sender<()>>>>,
}

impl GatewayAlbyClient {
    pub async fn new(listen_address: String, api_key: String) -> Self {
        info!("Gateway configured to connect to Alby at \n address: {listen_address:?}");
        Self {
            api_key,
            map: Arc::new(Mutex::new(BTreeMap::new())),
        }
    }
}

impl fmt::Debug for GatewayAlbyClient {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "AlbyClient")
    }
}

#[async_trait]
impl ILnRpcClient for GatewayAlbyClient {
    /// Returns the public key of the lightning node to use in route hint
    ///
    /// What should we do here with Alby?
    /// - Is the pubkey always the same?
    /// - Could change to optional: If Custodial API, hardcode the pubkey for
    ///   the node
    async fn info(&self) -> Result<GetNodeInfoResponse, LightningRpcError> {
        let mainnet = "mainnet";
        let alias = "getalby.com";
        let pub_key = PublicKey::from_str(
            "030a58b8653d32b99200a2334cfe913e51dc7d155aa0116c176657a4f1722677a3",
        )
        .unwrap();
        let pub_key = pub_key.serialize().to_vec();

        return Ok(GetNodeInfoResponse {
            pub_key: pub_key,
            alias: alias.to_string(),
            network: mainnet.to_string(),
        });
    }

    /// We can probably just use the Alby node pubkey here?
    /// SCID is the short channel ID mapping to the federation
    async fn routehints(
        &self,
        num_route_hints: usize,
    ) -> Result<GetRouteHintsResponse, LightningRpcError> {
        todo!()
        // Ok(GetRouteHintsResponse { route_hints })
    }

    /// Pay an invoice using the alby api
    /// Pay needs to be idempotent, this is why we need lookup payment,
    /// would need to do something similar with Alby
    async fn pay(
        &self,
        request: PayInvoiceRequest,
    ) -> Result<PayInvoiceResponse, LightningRpcError> {
        let client = reqwest::Client::new();
        let endpoint = "https://api.getalby.com/payments/bolt11";

        let req = json!({
            "invoice": request.invoice,
        });

        let response = client
            .post(endpoint)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&req)
            .send()
            .await
            .unwrap();

        let response = response.json::<AlbyPayResponse>().await.unwrap();

        Ok(PayInvoiceResponse {
            preimage: response.payment_preimage,
        })
    }

    // FIXME: deduplicate implementation with pay
    async fn pay_private(
        &self,
        invoice: PrunedInvoice,
        max_delay: u64,
        max_fee: Amount,
    ) -> Result<PayInvoiceResponse, LightningRpcError> {
        todo!()

        // Ok(PayInvoiceResponse { preimage })
    }

    /// Returns true if the lightning backend supports payments without full
    /// invoices
    fn supports_private_payments(&self) -> bool {
        false
    }

    /// TODO: For receiving payments, we need to implement a webhook
    async fn route_htlcs<'a>(
        self: Box<Self>,
        task_group: &mut TaskGroup,
    ) -> Result<(RouteHtlcStream<'a>, Arc<dyn ILnRpcClient>), LightningRpcError> {
        let (gateway_sender, gateway_receiver) =
            mpsc::channel::<Result<InterceptHtlcRequest, tonic::Status>>(CHANNEL_SIZE);

        let (lnd_sender, lnd_rx) = mpsc::channel::<ForwardHtlcInterceptResponse>(CHANNEL_SIZE);
        // Ok((Box::pin(ReceiverStream::new(gateway_receiver)), new_client))
    }

    /// TODO: For receiving payments, we need to implement a webhook
    async fn complete_htlc(
        &self,
        htlc: InterceptHtlcResponse,
    ) -> Result<EmptyResponse, LightningRpcError> {
        todo!()

        // Err(LightningRpcError::FailedToCompleteHtlc {
        //     failure_reason: "Gatewayd has not started to route
        // HTLCs".to_string(), })
    }
}
