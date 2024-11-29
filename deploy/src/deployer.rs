use fuels::core::Configurables;
use fuels::prelude::{
    abigen,
    Contract,
    ContractId,
    LoadConfiguration,
    TxPolicies,
    WalletUnlocked,
};
use maplit::hashmap;
use rand::Rng;
use std::future::Future;
use crate::fixture::Fixture;
use crate::shared::{config, get_wallets};
use fuelname_sdk::interface::{Registrar, Registry, Resolver, ResolverConfigurables, RegistrarConfigurables};

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum ContractType {
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
        name = "Proxy",
        abi = "proxy/out/release/proxy-abi.json"
    ),
);


#[derive(Debug, Clone)]
pub struct ProxiesInfo {
    pub registrar: ContractId,
    pub registry: ContractId,
    pub resolver: ContractId,
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

#[derive(Debug, Clone)]
pub enum DeployParams {
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

#[derive(Debug, Clone)]
pub struct DeployResult {
    pub target_id: ContractId,
    pub proxy_id: ContractId,
}

pub struct LocalDeployParams {
    pub deployer_wallet: WalletUnlocked,
    pub user_wallet: WalletUnlocked,
    pub deploy_params: DeployParams,
}

pub enum DeployTarget {
    Local(LocalDeployParams),
    OnChain,
}

pub async fn deploy(target: DeployTarget) -> Fixture {
    let (deployer_wallet, user_wallet, params) = match target {
        DeployTarget::Local(p) => {
            (p.deployer_wallet, p.user_wallet, p.deploy_params)
        }
        DeployTarget::OnChain => {
            let config = config();
            let params = config.clone().deploy_params;
            let (deployer, user) = get_wallets(&config).await;
            println!("Deployer wallet address: {:?}", deployer.address().hash);
            println!("Deploy params: {:#?}", params);
            (deployer, user, params)
        }
    };
    let registry = deploy_registry_contract(&deployer_wallet, &params).await;
    let resolver = deploy_resolver_contract(&deployer_wallet, &params, &registry).await;
    let registrar = deploy_registrar_contract(
        &deployer_wallet,
        &params,
        &registry,
        &resolver,
    ).await;
    let contracts = hashmap! {
        ContractType::Registry => registry.clone(),
        ContractType::Resolver => resolver.clone(),
        ContractType::Registrar => registrar.clone(),
    };
    println!("{:#?}", contracts);
    Fixture {
        deployer: deployer_wallet.clone(),
        user: user_wallet,
        registry_contract: Registry::new(registry.proxy_id, deployer_wallet.clone()),
        resolver_contract: Resolver::new(resolver.proxy_id, deployer_wallet.clone()),
        registrar_contract: Registrar::new(registrar.proxy_id, deployer_wallet),
        contracts,
    }
}

async fn _deploy(
    wallet: &WalletUnlocked,
    contract: &str,
    configurables: Option<Configurables>,
) -> ContractId {
    let mut rng = rand::thread_rng();
    let configurables = configurables.unwrap_or_default();
    let id = Contract::load_from(
        format!("../{}/out/release/{}.bin", contract, contract),
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
            deploy_proxy_contract(wallet, id.clone(), contract).await
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
        // call target initialize() directly so no one else can set the owner
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
        // call target initialize() directly so no one else can set the owner
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
    target_contract: &ContractType,
) -> ContractId {
    let id = _deploy(wallet, "proxy", None).await;
    let proxy = Proxy::new(id.clone(), wallet.clone());
    proxy.methods().initialize_proxy_ownership().call().await.unwrap();
    proxy.methods().set_proxy_target(proxy_target).call().await.unwrap();
    let owner = proxy.methods().proxy_owner().call().await.unwrap().value;
    println!("{}-proxy owner: {:?}", target_contract.name(), owner);
    id
}
