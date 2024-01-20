use std::collections::BTreeMap;
use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use fedimint_core::task::TaskGroup;
use fedimint_core::Amount;
use fedimint_ln_common::PrunedInvoice;
use lightning_invoice::Bolt11Invoice;
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::oneshot::Sender;
use tokio::sync::{mpsc, Mutex};
use tokio_stream::wrappers::ReceiverStream;
use tracing::info;

use super::{
    send_htlc_to_webhook, ILnRpcClient, LightningRpcError, RouteHtlcStream, WebhookClient,
};
use crate::gateway_lnrpc::{
    EmptyResponse, GetNodeInfoResponse, GetRouteHintsResponse, InterceptHtlcRequest,
    InterceptHtlcResponse, PayInvoiceRequest, PayInvoiceResponse,
};
use crate::rpc::rpc_webhook_server::run_webhook_server;

#[derive(Debug, Serialize, Deserialize)]
pub struct StrikeAmount {
    amount: String,
    currency: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrikeInvoiceResponse {
    invoice_id: String,
    amount: StrikeAmount,
    state: String,
    created: String,
    correlation_id: String,
    description: String,
    issuer_id: String,
    receiver_id: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversionRate {
    amount: String,
    source_currency: String,
    target_currency: String,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrikeQuoteResponse {
    quote_id: String,
    description: String,
    ln_invoice: Bolt11Invoice,
    onchain_address: String,
    expiration: String,
    expiration_in_sec: u32,
    source_amount: StrikeAmount,
    target_amount: StrikeAmount,
    conversion_rate: ConversionRate,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrikePaymentQuote {
    payment_quote_id: String,
    description: String,
    valid_until: String,
    conversion_rate: ConversionRate,
    amount: StrikeAmount,
    lighning_network_fee: StrikeAmount,
    total_amount: StrikeAmount,
}

#[derive(Clone)]
pub struct GatewayStrikeClient {
    bind_addr: SocketAddr,
    api_key: String,
    pub outcomes: Arc<Mutex<BTreeMap<u64, Sender<InterceptHtlcResponse>>>>,
}

impl GatewayStrikeClient {
    pub async fn new(
        bind_addr: SocketAddr,
        api_key: String,
        outcomes: Arc<Mutex<BTreeMap<u64, Sender<InterceptHtlcResponse>>>>,
    ) -> Self {
        info!("Gateway configured to connect to Strike at \n address: {bind_addr:?}");
        Self {
            api_key,
            bind_addr,
            outcomes,
        }
    }
}

impl fmt::Debug for GatewayStrikeClient {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "StrikeClient")
    }
}

#[async_trait]
impl ILnRpcClient for GatewayStrikeClient {
    /// Returns the public key of the lightning node to use in route hint
    /// Strike always uses the same pubkey, so we can get it by querying
    /// for an invoice and then parsing the pubkey and network
    async fn info(&self) -> Result<GetNodeInfoResponse, LightningRpcError> {
        let endpoint = "https://api.strike.me/v1/invoices";
        let alias = "StrikeZapHq";

        let client = reqwest::Client::new();
        let req = json!({
            "invoice": {
                "amount": 0.0000001,
                "currency": "BTC"
            },
            "description": "Gateway GetInfo",
        });
        let response = client
            .post(endpoint)
            .header("Content-Type", "application/json")
            .bearer_auth(&self.api_key)
            .json(&req)
            .send()
            .await
            .map_err(|e| LightningRpcError::FailedToGetInvoice {
                failure_reason: format!("Failed to get invoice: {:?}", e),
            })?;

        let invoice_response = response.json::<StrikeInvoiceResponse>().await.unwrap();

        let response = client
            .get(format!(
                "https://api.strike.me/v1/invoices/{}/quote",
                invoice_response.invoice_id
            ))
            .header("Content-Type", "application/json")
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| LightningRpcError::FailedToGetInvoice {
                failure_reason: format!("Failed to get invoice: {:?}", e),
            })?;

        let quote_response = response.json::<StrikeQuoteResponse>().await.unwrap();

        let pub_key = quote_response.ln_invoice.payee_pub_key().ok_or(
            LightningRpcError::FailedToGetInvoice {
                failure_reason: format!("Failed to get invoice: {:?}", "No payee pub key"),
            },
        )?;

        return Ok(GetNodeInfoResponse {
            pub_key: pub_key.serialize().to_vec(),
            alias: alias.to_string(),
            network: quote_response.ln_invoice.network().to_string(),
        });
    }

    /// We can probably just use the Coinos node pubkey here?
    /// SCID is the short channel ID mapping to the federation
    async fn routehints(
        &self,
        _num_route_hints: usize,
    ) -> Result<GetRouteHintsResponse, LightningRpcError> {
        todo!()
    }

    /// Pay an invoice using the Coinos Api
    async fn pay(
        &self,
        request: PayInvoiceRequest,
    ) -> Result<PayInvoiceResponse, LightningRpcError> {
        let endpoint = "https://api.strike.me/v1/payments";
        let client = reqwest::Client::new();
        let req = json!({
            "lnInvoice": request.invoice,
            "sourceCurrency": "BTC",
        });
        let response = client
            .post(endpoint)
            .header("Content-Type", "application/json")
            .bearer_auth(&self.api_key)
            .json(&req)
            .send()
            .await
            .map_err(|e| LightningRpcError::FailedPayment {
                failure_reason: format!("Failed to pay invoice: {:?}", e),
            })?;

        let payment_quote_response = response.json::<StrikePaymentQuote>().await.unwrap();

        let _response = client
            .post(format!(
                "https://api.strike.me/v1/payments/{}/execute",
                payment_quote_response.payment_quote_id
            ))
            .header("Content-Type", "application/json")
            .bearer_auth(&self.api_key)
            .send()
            .await
            .map_err(|e| LightningRpcError::FailedPayment {
                failure_reason: format!("Failed to pay invoice: {:?}", e),
            })?;

        // TODO: Strike payment quote execution doesn't return a preimage ??????

        Ok(PayInvoiceResponse { preimage: vec![] })
    }

    // FIXME: deduplicate implementation with pay
    async fn pay_private(
        &self,
        _invoice: PrunedInvoice,
        _max_delay: u64,
        _max_fee: Amount,
    ) -> Result<PayInvoiceResponse, LightningRpcError> {
        todo!()

        // Ok(PayInvoiceResponse { preimage })
    }

    /// Returns true if the lightning backend supports payments without full
    /// invoices
    fn supports_private_payments(&self) -> bool {
        false
    }

    async fn route_htlcs<'a>(
        self: Box<Self>,
        task_group: &mut TaskGroup,
    ) -> Result<(RouteHtlcStream<'a>, Arc<dyn ILnRpcClient>), LightningRpcError> {
        const CHANNEL_SIZE: usize = 100;
        let (gateway_sender, gateway_receiver) =
            mpsc::channel::<Result<InterceptHtlcRequest, tonic::Status>>(CHANNEL_SIZE);

        let new_client =
            Arc::new(Self::new(self.bind_addr, self.api_key.clone(), self.outcomes.clone()).await);

        run_webhook_server(
            self.bind_addr,
            task_group,
            gateway_sender.clone(),
            WebhookClient::Strike(*self),
        )
        .await
        .map_err(|_| LightningRpcError::FailedToRouteHtlcs {
            failure_reason: "Failed to start webhook server".to_string(),
        })?;

        Ok((Box::pin(ReceiverStream::new(gateway_receiver)), new_client))
    }

    async fn complete_htlc(
        &self,
        htlc: InterceptHtlcResponse,
    ) -> Result<EmptyResponse, LightningRpcError> {
        send_htlc_to_webhook(&self.outcomes, htlc).await?;
        Ok(EmptyResponse {})
    }
}
