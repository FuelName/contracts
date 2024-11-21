use std::str::FromStr;
use dotenvy::dotenv;
use fuels::crypto::SecretKey;
use fuels::prelude::{Provider, WalletUnlocked};
use crate::deployer::{DeployParams, ProxiesInfo};

#[derive(Clone)]
pub struct Config {
    pub fuel_url: String,
    pub deployer_pk: String,
    pub user_pk: String,
    pub deploy_params: DeployParams,
}

pub fn config() -> Config {
    dotenv().unwrap();
    let deploy_mode = std::env::var("DEPLOY_MODE").expect("missing env var DEPLOY_MODE");
    let deploy_params = match deploy_mode.as_str() {
        "INITIAL" => DeployParams::InitialDeploy,
        "UPGRADE" => DeployParams::Upgrade(
            ProxiesInfo {
                registrar: std::env::var("REGISTRAR_PROXY").expect("missing env var REGISTRAR_PROXY").parse().unwrap(),
                registry: std::env::var("REGISTRY_PROXY").expect("missing env var REGISTRY_PROXY").parse().unwrap(),
                resolver: std::env::var("RESOLVER_PROXY").expect("missing env var RESOLVER_PROXY").parse().unwrap(),
            }
        ),
        _ => {
            panic!("Invalid deploy mode: {}, must be INITIAL or UPGRADE", deploy_mode);
        }
    };
    Config {
        fuel_url: std::env::var("FUEL_URL").expect("missing env var FUEL_URL"),
        deployer_pk: std::env::var("DEPLOYER_PK").expect("missing env var DEPLOYER_PK"),
        user_pk: std::env::var("USER_PK").expect("missing env var USER_PK"),
        deploy_params,
    }
}

pub async fn get_wallets(
    config: &Config
) -> (WalletUnlocked, WalletUnlocked) {
    let deployer_pk = SecretKey::from_str(&config.deployer_pk).unwrap();
    let user_pk = SecretKey::from_str(&config.user_pk).unwrap();
    let provider = Provider::connect(&config.fuel_url).await.unwrap();
    (
        WalletUnlocked::new_from_private_key(deployer_pk, Some(provider.clone())),
        WalletUnlocked::new_from_private_key(user_pk, Some(provider.clone())),
    )
}
