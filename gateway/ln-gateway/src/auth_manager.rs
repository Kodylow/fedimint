use std::collections::BTreeSet;
use std::str::FromStr;
use std::time::UNIX_EPOCH;

use bitcoin_hashes::{sha256, Hash};
use fedimint_core::encoding::Encodable;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, TokenData, Validation};
use reqwest::StatusCode;
use secp256k1_zkp::schnorr::Signature;
use secp256k1_zkp::PublicKey;
use serde::{Deserialize, Serialize};

const CHALLENGE_EXPIRY_SECONDS: u64 = 60; // 1 minute

pub enum AuthToken {
    TokenData(TokenData<Session>),
    HashedPassword(String),
}

pub struct AuthManager {
    /// The public key of the gateway
    gateway_id: PublicKey,
    /// The API endpoint of the gateway
    gateway_api: String,
    /// Challenges with their expiry times
    challenges: BTreeSet<AuthChallenge>,
    /// The JWT secret
    jwt_secret: String,
}

impl AuthManager {
    /// Create a new auth manager
    pub fn new(gateway_id: PublicKey, gateway_api: String, jwt_secret: String) -> Self {
        Self {
            gateway_id,
            gateway_api,
            challenges: BTreeSet::new(),
            jwt_secret,
        }
    }

    /// Create a new challenge
    pub fn create_challenge(&mut self) -> AuthChallenge {
        let challenge = AuthChallenge::new(self.gateway_id.clone(), self.gateway_api.clone());
        self.challenges.insert(challenge.clone());
        challenge
    }

    /// Verify the challenge response and return a JWT session if valid
    pub fn verify_challenge_response(
        &mut self,
        ctx: &secp256k1_zkp::Secp256k1<secp256k1_zkp::All>,
        challenge_response: &AuthChallengeResponse,
    ) -> anyhow::Result<String> {
        let challenge = AuthChallenge::from_str(&challenge_response.challenge)?;
        //TODO check password and if correct generate Session
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
        let jwt = self.encode_jwt()?;
        Ok(jwt)
    }

    pub fn get_auth_token(&self, token: &str) -> Result<AuthToken, StatusCode> {
        if let Ok(token_data) = decode::<Session>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_ref()),
            &Validation::default(),
        ) {
            return Ok(AuthToken::TokenData(token_data));
        }
        Ok(AuthToken::HashedPassword(token.to_string()))
    }

    pub fn encode_jwt(&self) -> anyhow::Result<String> {
        let session = Session::new();
        encode(
            &Header::default(),
            &session,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        )
        .map_err(|_| anyhow::anyhow!("Unable to generate jwt token session"))
    }

    /// Validate that the Bearer token matches the gateway's hashed password
    async fn authenticate_password(
        gateway_hashed_password: sha256::Hash,
        password_salt: [u8; 16],
        token: String,
    ) -> Result<(), StatusCode> {
        let hashed_password = hash_password(&token, password_salt);
        if gateway_hashed_password == hashed_password {
            return Ok(());
        }
        Err(StatusCode::UNAUTHORIZED)
    }

    pub fn authenticate_jwt_expiry(
        &self,
        token_data: &TokenData<Session>,
    ) -> Result<(), StatusCode> {
        if token_data.claims.expiry
            < fedimint_core::time::now()
                .duration_since(UNIX_EPOCH)
                .expect("Time went backwards")
                .as_secs()
        {
            return Err(StatusCode::UNAUTHORIZED);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct AuthChallenge {
    gateway_id: PublicKey,
    gateway_api: String,
    /// The expiry time of the challenge
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
    /// The challenge string
    challenge: String,
    /// The response string
    response: Signature,
    /// optional password for backwards compatibility
    password: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Session {
    /// the unique identifier of the session.
    pub id: String,
    /// the expire time of the session
    pub expiry: u64,
}

impl Session {
    pub fn new() -> Self {
        let id = rand::random::<u64>().to_string();
        let expiry = fedimint_core::time::now()
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs()
            + CHALLENGE_EXPIRY_SECONDS;
        Self { id, expiry }
    }
}

/// Creates a password hash by appending a 4 byte salt to the plaintext
/// password.
pub fn hash_password(plaintext_password: &str, salt: [u8; 16]) -> sha256::Hash {
    let mut bytes = Vec::<u8>::new();
    plaintext_password
        .consensus_encode(&mut bytes)
        .expect("Password is encodable");
    salt.consensus_encode(&mut bytes)
        .expect("Salt is encodable");
    sha256::Hash::hash(&bytes)
}
