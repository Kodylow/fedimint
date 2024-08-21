use std::collections::BTreeSet;
use std::str::FromStr;
use std::time::UNIX_EPOCH;

use secp256k1_zkp::schnorr::Signature;
use secp256k1_zkp::PublicKey;
use serde::{Deserialize, Serialize};

const CHALLENGE_EXPIRY_SECONDS: u64 = 60; // 1 minute

pub struct AuthManager {
    /// The public key of the gateway.
    gateway_id: PublicKey,
    /// The API endpoint of the gateway.
    gateway_api: String,
    /// Challenges with their expiry times.
    challenges: BTreeSet<AuthChallenge>,
}

impl AuthManager {
    /// Create a new auth manager.
    pub fn new(gateway_id: PublicKey, gateway_api: String) -> Self {
        Self {
            gateway_id,
            gateway_api,
            challenges: BTreeSet::new(),
        }
    }

    /// Create a new challenge.
    pub fn create_challenge(&mut self) -> AuthChallenge {
        let challenge = AuthChallenge::new(self.gateway_id.clone(), self.gateway_api.clone());
        self.challenges.insert(challenge.clone());
        challenge
    }

    /// Verify the challenge.
    pub fn verify_challenge_response(
        &mut self,
        ctx: &secp256k1_zkp::Secp256k1<secp256k1_zkp::All>,
        challenge_response: &AuthChallengeResponse,
    ) -> anyhow::Result<()> {
        let challenge = AuthChallenge::from_str(&challenge_response.challenge)?;
        if !self.challenges.contains(&challenge)
            || challenge.gateway_id != self.gateway_id
            || challenge.gateway_api != self.gateway_api
        {
            return Err(anyhow::anyhow!("Invalid challenge"));
        }
        if challenge.expiry
            < fedimint_core::time::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs()
        {
            return Err(anyhow::anyhow!("Challenge expired"));
        }

        // Verify the schnorr signature against the gateway's pubkey
        let message = secp256k1_zkp::Message::from_slice(challenge.to_string().as_bytes())?;
        let signature = challenge_response.response;
        ctx.verify_schnorr(&signature, &message, &self.gateway_id.x_only_public_key().0)
            .map_err(|_| anyhow::anyhow!("Invalid signature"))?;

        // If valid, remove the challenge from the set
        self.challenges.remove(&challenge);
        // TODO: Create a session token
        Ok(())
    }
}

#[derive(Debug, Deserialize, Clone, Serialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct AuthChallenge {
    gateway_id: PublicKey,
    gateway_api: String,
    /// The expiry time of the challenge.
    expiry: u64,
}

/// Structure to represent a challenge.
/// gatewayid-gatewayapi-timestamp
impl AuthChallenge {
    /// Create a new challenge with a random string and an expiry time of 1
    /// minute.
    pub fn new(gateway_id: PublicKey, gateway_api: String) -> Self {
        let now = fedimint_core::time::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();
        let expiry = now + CHALLENGE_EXPIRY_SECONDS;
        Self {
            gateway_id,
            gateway_api,
            expiry,
        }
    }
}

impl FromStr for AuthChallenge {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts = s.split('-').collect::<Vec<&str>>();
        let gateway_id = PublicKey::from_str(parts[0])?;
        let gateway_api = parts[1].to_string();
        let expiry = parts[2].parse::<u64>()?;
        Ok(Self {
            gateway_id,
            gateway_api,
            expiry,
        })
    }
}

impl ToString for AuthChallenge {
    fn to_string(&self) -> String {
        format!("{}-{}-{}", self.gateway_id, self.gateway_api, self.expiry)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AuthChallengeResponse {
    /// The challenge string.
    challenge: String,
    /// The response string.
    response: Signature,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Session {
    /// the unique identifier of the session.
    id: String,
    /// The expiry time of the session.
    expiry: u64,
}
