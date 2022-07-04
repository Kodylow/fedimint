use bitcoin::secp256k1::PublicKey;
use bitcoin::XOnlyPublicKey;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LightningGateway {
    pub mint_pub_key: XOnlyPublicKey,
    pub node_pub_key: PublicKey,
    pub api: String,
}
