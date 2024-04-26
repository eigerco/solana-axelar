use std::net::SocketAddr;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::Arc;

use anyhow::{bail, Ok};
use clap::Parser;
use figment::providers::Env;
use figment::Figment;
use serde::{Deserialize, Deserializer};
use solana_program::pubkey::Pubkey;
use solana_sdk::signature::Keypair;
use solana_sdk::signer::Signer;
use tonic::transport::Uri;
use url::Url;

/// Solana network name used to identify it on Axelar.
pub const SOLANA_CHAIN_NAME: &str = "solana";

/// Solana GMP Gateway root config PDA

#[derive(Parser, Debug, Clone)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the configuration file
    #[arg(long)]
    pub config: PathBuf,
}

#[derive(Deserialize)]
#[cfg_attr(test, derive(Debug))]
pub struct ConfigEnv {
    pub database_url: Url,
    pub axelar_approver_url: Url,
    pub solana_includer_rpc: Url,
    #[serde(deserialize_with = "deserialize_keypair")]
    pub solana_includer_keypair: Arc<Keypair>,
    #[serde(deserialize_with = "deserialize_pubkey")]
    pub sentinel_gateway_address: Pubkey,
    #[serde(deserialize_with = "deserialize_pubkey")]
    pub sentinel_gateway_config_address: Pubkey,
    pub sentinel_rpc: Url,
    #[serde(deserialize_with = "deserialize_uri")]
    pub verifier_rpc: Uri,
    #[serde(deserialize_with = "deserialize_socket_addr")]
    pub healthcheck_bind_addr: SocketAddr,
}

impl ConfigEnv {
    pub fn load() -> Result<Self, figment::Error> {
        Figment::new().merge(Env::prefixed("RELAYER_")).extract()
    }
}

#[derive(Deserialize, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct Config {
    pub axelar_to_solana: Option<AxelarToSolana>,
    pub solana_to_axelar: Option<SolanaToAxelar>,
    pub database: Database,
    pub healthcheck_bind_addr: SocketAddr,
}

impl Config {
    pub fn validate(&self) -> anyhow::Result<()> {
        let directions = (&self.axelar_to_solana, &self.solana_to_axelar);

        if let (None, None) = directions {
            bail!("Relayer must be configured with at least one message transport direction")
        }

        if let Some(_axelar_to_solana) = &self.axelar_to_solana {}
        if let Some(_solana_to_axelar) = &self.solana_to_axelar {
            // Put relevant validation logic here
        }

        Ok(())
    }

    pub fn from_env() -> anyhow::Result<Config> {
        let config = ConfigEnv::load()?;
        Ok(Config {
            axelar_to_solana: Some(AxelarToSolana {
                approver: AxelarApprover {
                    rpc: config.axelar_approver_url,
                    relayer_account: config.solana_includer_keypair.pubkey(),
                },
                includer: SolanaIncluder {
                    rpc: config.solana_includer_rpc,
                    keypair: config.solana_includer_keypair,
                    gateway_address: config.sentinel_gateway_address,
                    gateway_config_address: config.sentinel_gateway_config_address,
                },
            }),
            solana_to_axelar: Some(SolanaToAxelar {
                sentinel: SolanaSentinel {
                    gateway_address: config.sentinel_gateway_address,
                    rpc: config.sentinel_rpc,
                },
                verifier: AxelarVerifier {
                    rpc: config.verifier_rpc,
                },
            }),
            database: Database {
                url: config.database_url,
            },
            healthcheck_bind_addr: config.healthcheck_bind_addr,
        })
    }
}

#[derive(Deserialize, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct AxelarToSolana {
    pub approver: AxelarApprover,
    pub includer: SolanaIncluder,
}

#[derive(Deserialize, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct SolanaToAxelar {
    pub sentinel: SolanaSentinel,
    pub verifier: AxelarVerifier,
}

#[derive(Deserialize, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct Database {
    pub url: Url,
}

#[derive(Deserialize, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct AxelarApprover {
    pub rpc: Url,
    pub relayer_account: Pubkey,
}

#[derive(Deserialize, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct SolanaIncluder {
    pub rpc: Url,
    #[serde(deserialize_with = "deserialize_keypair")]
    pub keypair: Arc<Keypair>,
    pub gateway_address: Pubkey,
    pub gateway_config_address: Pubkey,
}

#[derive(Deserialize, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct SolanaSentinel {
    #[serde(deserialize_with = "deserialize_pubkey")]
    pub gateway_address: Pubkey,
    pub rpc: Url,
}

#[derive(Deserialize, PartialEq)]
#[cfg_attr(test, derive(Debug))]
pub struct AxelarVerifier {
    #[serde(deserialize_with = "deserialize_uri")]
    pub rpc: Uri,
}

fn deserialize_keypair<'de, D>(deserializer: D) -> Result<Arc<Keypair>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    let bytes = solana_sdk::bs58::decode(s)
        .into_vec()
        .map_err(serde::de::Error::custom)?;
    Keypair::from_bytes(&bytes)
        .map(Arc::new)
        .map_err(serde::de::Error::custom)
}

fn deserialize_pubkey<'de, D>(deserializer: D) -> Result<Pubkey, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Pubkey::from_str(&s).map_err(serde::de::Error::custom)
}

fn deserialize_socket_addr<'de, D>(deserializer: D) -> Result<SocketAddr, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    SocketAddr::from_str(&s).map_err(serde::de::Error::custom)
}

fn deserialize_uri<'de, D>(deserializer: D) -> Result<Uri, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Uri::from_str(&s).map_err(serde::de::Error::custom)
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;

    #[test]
    fn can_parse_config_from_env() {
        let db_url = "http://0.0.0.0/";
        let approver_url = "http://0.0.0.1/";
        let includer_rpc = "http://0.0.0.2/";
        let gw_addr = "4hz16cS4d82cPKzvaQNzMCadyKSqzZR8bqzw8FfzYH8a";
        let gw_config_addr = "11fDsmgXjuUtAVC6PXveUUrUgSVTNC4BMMmsD9AUdUB";
        let sentinel_rpc = "http://0.0.0.3/";
        let verifier_rpc = "http://0.0.0.4/";
        let keypair = Arc::new(Keypair::new());
        let healthcheck_bind_addr = "127.0.0.1:3000";

        env::set_var("RELAYER_DATABASE_URL", db_url);
        env::set_var("RELAYER_AXELAR_APPROVER_URL", approver_url);
        env::set_var("RELAYER_SOLANA_INCLUDER_RPC", includer_rpc);
        env::set_var(
            "RELAYER_SOLANA_INCLUDER_KEYPAIR",
            keypair.to_base58_string(),
        );
        env::set_var("RELAYER_SENTINEL_GATEWAY_ADDRESS", gw_addr);
        env::set_var("RELAYER_SENTINEL_GATEWAY_CONFIG_ADDRESS", gw_config_addr);
        env::set_var("RELAYER_SENTINEL_RPC", sentinel_rpc);
        env::set_var("RELAYER_VERIFIER_RPC", verifier_rpc);
        env::set_var("RELAYER_HEALTHCHECK_BIND_ADDR", healthcheck_bind_addr);

        assert_eq!(
            Config::from_env().unwrap(),
            Config {
                axelar_to_solana: Some(AxelarToSolana {
                    approver: AxelarApprover {
                        rpc: Url::from_str(approver_url).unwrap(),
                        relayer_account: keypair.pubkey(),
                    },
                    includer: SolanaIncluder {
                        rpc: Url::from_str(includer_rpc).unwrap(),
                        keypair,
                        gateway_address: Pubkey::from_str(gw_addr).unwrap(),
                        gateway_config_address: Pubkey::from_str(gw_config_addr).unwrap(),
                    },
                }),
                solana_to_axelar: Some(SolanaToAxelar {
                    sentinel: SolanaSentinel {
                        gateway_address: Pubkey::from_str(gw_addr).unwrap(),
                        rpc: Url::from_str(sentinel_rpc).unwrap(),
                    },
                    verifier: AxelarVerifier {
                        rpc: Uri::from_str(verifier_rpc).unwrap()
                    },
                }),
                database: Database {
                    url: Url::from_str(db_url).unwrap()
                },
                healthcheck_bind_addr: SocketAddr::from_str(healthcheck_bind_addr).unwrap()
            }
        );
    }
}
