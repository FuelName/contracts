use dotenvy::dotenv;
use fuels::core::Configurables;
use fuels::crypto::SecretKey;
use fuels::prelude::{
    abigen,
    Contract,
    ContractId,
    LoadConfiguration,
    Provider,
    TxPolicies,
    WalletUnlocked,
};
use maplit::hashmap;
use rand::Rng;
use std::collections::HashMap;
use std::future::Future;
use std::str::FromStr;

#[tokio::main]
async fn main() {
    let deploy_result = deploy().await;
    println!("{:#?}", deploy_result);
}

#[derive(Debug, PartialEq, Eq, Hash)]
enum ContractType {
    Registrar,
    Registry,
    Resolver,
}

impl ContractType {
    fn name(&self) -> &str {
        match self {
            ContractType::Registrar => "registrar",
            ContractType::Registry => "registry",
            ContractType::Resolver => "resolver",
        }
    }
}

abigen!(
    Contract(
        name = "Registrar",
        abi = "registrar/out/debug/registrar-abi.json"
    ),
    Contract(
        name = "Registry",
        abi = "registry/out/debug/registry-abi.json"
    ),
    Contract(
        name = "Resolver",
        abi = "resolver/out/debug/resolver-abi.json"
    ),
    Contract(
        name = "Proxy",
        abi = "proxy/out/debug/proxy-abi.json"
    ),
);

struct Config {
    fuel_url: String,
    private_key: String,
    deploy_params: DeployParams,
}

#[derive(Debug)]
struct ProxiesInfo {
    registrar: ContractId,
    registry: ContractId,
    resolver: ContractId,
}

impl ProxiesInfo {
    fn proxy_of(&self, contract_type: &ContractType) -> ContractId {
        match contract_type {
            ContractType::Registrar => self.registrar,
            ContractType::Registry => self.registry,
            ContractType::Resolver => self.resolver,
        }
    }
}

#[derive(Debug)]
enum DeployParams {
    InitialDeploy,
    Upgrade(ProxiesInfo),
}

impl DeployParams {
    fn is_initial(&self) -> bool {
        match self {
            DeployParams::InitialDeploy => true,
            _ => false,
        }
    }
}

fn config() -> Config {
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
        private_key: std::env::var("PRIVATE_KEY").expect("missing env var PRIVATE_KEY"),
        deploy_params,
    }
}

#[derive(Debug, Clone)]
struct DeployResult {
    target_id: ContractId,
    proxy_id: ContractId,
}

async fn deploy() -> HashMap<ContractType, DeployResult> {
    let config = config();
    let params = &config.deploy_params;
    let wallet = get_wallet(&config).await;
    println!("Deployer wallet address: {:?}", wallet.address().hash);
    println!("Deploy params: {:#?}", params);
    let registry = deploy_registry_contract(&wallet, params).await;
    let resolver = deploy_resolver_contract(&wallet, params, &registry).await;
    let registrar = deploy_registrar_contract(
        &wallet,
        params,
        &registry,
        &resolver,
    ).await;
    hashmap! {
        ContractType::Registry => registry,
        ContractType::Resolver => resolver,
        ContractType::Registrar => registrar,
    }
}

async fn get_wallet(
    config: &Config
) -> WalletUnlocked {
    let secret_key = SecretKey::from_str(&config.private_key).unwrap();
    let provider = Provider::connect(&config.fuel_url).await.unwrap();
    WalletUnlocked::new_from_private_key(secret_key, Some(provider.clone()))
}

async fn _deploy(
    wallet: &WalletUnlocked,
    contract: &str,
    configurables: Option<Configurables>,
) -> ContractId {
    let mut rng = rand::thread_rng();
    let configurables = configurables.unwrap_or_default();
    let id = Contract::load_from(
        format!("../{}/out/debug/{}.bin", contract, contract),
        LoadConfiguration::default().with_configurables(configurables),
    )
        .unwrap()
        .with_salt(rng.gen::<[u8; 32]>())
        .deploy(wallet, TxPolicies::default())
        .await
        .unwrap()
        .into();
    id
}

async fn deploy_with_proxy<F, R>(
    wallet: &WalletUnlocked,
    contract: &ContractType,
    configurables: Option<Configurables>,
    deploy_params: &DeployParams,
    init: F,
) -> DeployResult where
    F: Fn(DeployResult) -> R,
    R: Future<Output=()>,
{
    let id = _deploy(wallet, contract.name(), configurables).await;
    let proxy_id = match deploy_params {
        DeployParams::InitialDeploy => {
            deploy_proxy_contract(wallet, id.clone()).await
        }
        DeployParams::Upgrade(proxies) => {
            let proxy_id = proxies.proxy_of(contract);
            let proxy = Proxy::new(proxy_id, wallet.clone());
            proxy.methods().set_proxy_target(id).call().await.unwrap();
            proxy_id
        }
    };
    let deploy_result = DeployResult {
        target_id: id,
        proxy_id,
    };
    init(deploy_result.clone()).await;
    deploy_result
}

async fn deploy_registry_contract(
    wallet: &WalletUnlocked,
    deploy_params: &DeployParams,
) -> DeployResult {
    println!("Deploying registry contract...");
    let init = |deploy: DeployResult| async move {
        let contract = Registry::new(deploy.target_id, wallet.clone());
        let owner = contract.methods()
            .initialize()
            .call()
            .await
            .unwrap()
            .value;
        println!("Registry owner (called directly): {:?}", owner);
        if deploy_params.is_initial() {
            let contract = Registry::new(deploy.proxy_id, wallet.clone());
            let owner = contract.methods()
                .initialize()
                .with_contract_ids(&[deploy.target_id.into()])
                .call()
                .await
                .unwrap()
                .value;
            println!("Registry owner (called through proxy): {:?}", owner);
        }
    };
    deploy_with_proxy(
        wallet,
        &ContractType::Registry,
        None,
        deploy_params,
        init,
    ).await
}

async fn skip_init(_: DeployResult) {}

async fn deploy_resolver_contract(
    wallet: &WalletUnlocked,
    deploy_params: &DeployParams,
    registry: &DeployResult,
) -> DeployResult {
    println!("Deploying resolver contract...");
    let configurables = ResolverConfigurables::default()
        .with_REGISTRY_CONTRACT_ID(registry.proxy_id)
        .unwrap();
    deploy_with_proxy(
        wallet,
        &ContractType::Resolver,
        Some(configurables.into()),
        deploy_params,
        skip_init,
    ).await
}

async fn deploy_registrar_contract(
    wallet: &WalletUnlocked,
    deploy_params: &DeployParams,
    registry: &DeployResult,
    resolver: &DeployResult,
) -> DeployResult {
    println!("Deploying registrar contract...");
    let configurables = RegistrarConfigurables::default()
        .with_REGISTRY_CONTRACT_ID(registry.proxy_id)
        .unwrap()
        .with_DEFAULT_RESOLVER_CONTRACT_ID(resolver.proxy_id)
        .unwrap();
    let init = |registrar: DeployResult| async move {
        let owner = Registrar::new(registrar.target_id, wallet.clone())
            .methods()
            .initialize()
            .call()
            .await
            .unwrap()
            .value;
        println!("Registrar owner (called directly): {:?}", owner);
        if deploy_params.is_initial() {
            let registrar_contract = Registrar::new(registrar.proxy_id, wallet.clone());
            let registry_contract = Registry::new(registry.proxy_id, wallet.clone());
            let owner = registrar_contract.methods()
                .initialize()
                .with_contract_ids(&[registrar.target_id.into()])
                .call()
                .await
                .unwrap()
                .value;
            println!("Registrar owner (called through proxy): {:?}", owner);
            let high_level_domain_asset = registry_contract
                .methods()
                .register_high_level_domain(registrar.proxy_id.into(), "fuel".to_string())
                .with_contract_ids(&[
                    registry.target_id.into(),
                    registrar.proxy_id.into(),
                ])
                .call()
                .await
                .unwrap()
                .value;
            println!("High level domain asset: {:?}", high_level_domain_asset);
        }
    };
    deploy_with_proxy(
        wallet,
        &ContractType::Registrar,
        Some(configurables.into()),
        deploy_params,
        init,
    ).await
}

async fn deploy_proxy_contract(
    wallet: &WalletUnlocked,
    proxy_target: ContractId,
) -> ContractId {
    let id = _deploy(wallet, "proxy", None).await;
    let proxy = Proxy::new(id.clone(), wallet.clone());
    proxy.methods().initialize_proxy_ownership().call().await.unwrap();
    proxy.methods().set_proxy_target(proxy_target).call().await.unwrap();
    id
}

