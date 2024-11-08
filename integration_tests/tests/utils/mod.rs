use std::str::FromStr;

use chrono::Duration;
use fuels::{accounts::wallet::WalletUnlocked, prelude::*};
use fuels::crypto::SecretKey;
use fuels::types::Identity;
use rand::Rng;

pub const THREE_LETTER_ANNUAL_DEFAULT_FEE: u64 = 50000000;
pub const FOUR_LETTER_ANNUAL_DEFAULT_FEE: u64 = 10000000;
pub const COMMON_ANNUAL_DEFAULT_FEE: u64 = 1000000;
pub const MIN_GRACE_PERIOD_DURATION: u64 = 2592000; // 30 days
pub const ONE_YEAR_SECONDS: u64 = 31622400;

pub const BASE_ASSET_ID: AssetId = AssetId::BASE;
pub fn usdc_asset_id() -> AssetId {
    AssetId::from_str("0x286c479da40dc953bddc3bb4c453b608bba2e0ac483b077bd475174115395e6b").unwrap()
}

const REGISTRY_DEPLOYED_CONTRACT_ID: &str =
    "0x8e66c1787462dad4193ce687eab081adbcbced4b2cc4170f061285a4489855e7";
const RESOLVER_DEPLOYED_CONTRACT_ID: &str =
    "0x41771453899a2170cfed89470dd414ce753e4d3b5b9c4f34e28a6e07e80425fe";
const REGISTRAR_DEPLOYED_CONTRACT_ID: &str =
    "0xbb04e3c7222d3bbcee2dda9bcc6ee4635235a9ac8d084489a435f448cc7b4a05";

const DEPLOYER_PK: &str = "don't commit";
const USER_PK: &str = "don't commit";

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
);

pub async fn get_wallets(on_chain: bool) -> (WalletUnlocked, WalletUnlocked) {
    if on_chain {
        get_on_chain_wallets(DEPLOYER_PK, USER_PK).await
    } else {
        get_custom_wallets().await
    }
}

async fn get_custom_wallets() -> (WalletUnlocked, WalletUnlocked) {
    let wallets = launch_custom_provider_and_get_wallets(
        WalletsConfig::new_multiple_assets(
            2,
            vec![
                AssetConfig {
                    id: BASE_ASSET_ID,
                    num_coins: 2,
                    coin_amount: 1_000_000_000,
                },
                AssetConfig {
                    id: usdc_asset_id(),
                    num_coins: 2,
                    coin_amount: 1_000_000_000,
                }
            ],
        ),
        None,
        None,
    )
        .await
        .unwrap();
    (wallets[0].clone(), wallets[1].clone())
}

async fn get_on_chain_wallets(
    deployer_pk: &str,
    user_pk: &str,
) -> (WalletUnlocked, WalletUnlocked) {
    let deployer_secret_key = SecretKey::from_str(deployer_pk).unwrap();
    let user_secret_key = SecretKey::from_str(user_pk).unwrap();
    let provider = Provider::connect("https://testnet.fuel.network")
        .await
        .unwrap();

    let deployer =
        WalletUnlocked::new_from_private_key(deployer_secret_key, Some(provider.clone()));
    let user = WalletUnlocked::new_from_private_key(user_secret_key, Some(provider.clone()));
    (deployer, user)
}

pub async fn get_registry_contract_instance(wallet: &WalletUnlocked) -> Registry<WalletUnlocked> {
    let mut rng = rand::thread_rng();
    let id = Contract::load_from(
        "../registry/out/debug/registry.bin",
        LoadConfiguration::default(),
    )
        .unwrap()
        .with_salt(rng.gen::<[u8; 32]>())
        .deploy(wallet, TxPolicies::default())
        .await
        .unwrap();
    print_contract_id("Registry", id.clone().into());
    let contract = Registry::new(id, wallet.clone());
    contract.methods().initialize().call().await.unwrap();
    contract
}

pub async fn get_resolver_contract_instance(
    wallet: &WalletUnlocked,
    registry_contract: &Registry<WalletUnlocked>,
) -> Resolver<WalletUnlocked> {
    let mut rng = rand::thread_rng();
    let configurables = ResolverConfigurables::default()
        .with_REGISTRY_CONTRACT_ID(registry_contract.id().into())
        .unwrap();
    let id = Contract::load_from(
        "../resolver/out/debug/resolver.bin",
        LoadConfiguration::default().with_configurables(configurables),
    )
        .unwrap()
        .with_salt(rng.gen::<[u8; 32]>())
        .deploy(wallet, TxPolicies::default())
        .await
        .unwrap();
    print_contract_id("Resolver", id.clone().into());
    Resolver::new(id, wallet.clone())
}

pub async fn get_registrar_contract_instance(
    wallet: &WalletUnlocked,
    registry_contract: &Registry<WalletUnlocked>,
    resolver_contract: &Resolver<WalletUnlocked>,
) -> (Registrar<WalletUnlocked>, AssetId) {
    let mut rng = rand::thread_rng();
    let configurables = RegistrarConfigurables::default()
        .with_REGISTRY_CONTRACT_ID(registry_contract.id().into())
        .unwrap()
        .with_DEFAULT_RESOLVER_CONTRACT_ID(resolver_contract.id().into())
        .unwrap();
    let id: Bech32ContractId = Contract::load_from(
        "../registrar/out/debug/registrar.bin",
        LoadConfiguration::default().with_configurables(configurables),
    )
        .unwrap()
        .with_salt(rng.gen::<[u8; 32]>())
        .deploy(wallet, TxPolicies::default())
        .await
        .unwrap();
    print_contract_id("Registrar", id.clone().into());
    let contract = Registrar::new(id.clone(), wallet.clone());
    contract.methods().initialize().call().await.unwrap();

    let high_level_domain_asset = registry_contract
        .methods()
        .register_high_level_domain(id.clone().into(), "fuel".to_string())
        .with_contract_ids(&[id.clone()])
        .call()
        .await
        .unwrap()
        .value;

    (contract, high_level_domain_asset)
}

pub struct Fixture {
    pub deployer: WalletUnlocked,
    pub user: WalletUnlocked,
    pub registry_contract: Registry<WalletUnlocked>,
    pub resolver_contract: Resolver<WalletUnlocked>,
    pub registrar_contract: Registrar<WalletUnlocked>,
    pub high_level_domain_asset: AssetId,
}

impl Fixture {
    pub async fn domain_exists(&self, asset_id: AssetId) -> bool {
        self.registry_contract
            .methods()
            .domain_exists(asset_id)
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value
    }

    pub async fn get_domain_asset_id(&self, domain: &str) -> AssetId {
        self.registry_contract
            .methods()
            .get_domain_asset_id(domain.to_string())
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value
    }

    pub async fn get_domain_name(&self, asset: AssetId) -> String {
        self.registry_contract
            .methods()
            .get_domain_name(asset)
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value
    }

    pub async fn get_token_uri(&self, asset: AssetId) -> Option<Metadata> {
        let metadata: Option<Metadata> = self
            .registry_contract
            .methods()
            .metadata(asset, "tokenURI".to_string())
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value;
        metadata
    }

    pub async fn set_domain_resolver(&self, domain: &str, resolver: ContractId) {
        let asset_id: AssetId = self
            .registry_contract
            .methods()
            .get_domain_asset_id(domain.to_string())
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value;
        self.registry_contract
            .clone()
            .with_account(self.user.clone())
            .methods()
            .set_resolver(domain.to_string(), resolver)
            .add_custom_asset(asset_id, 1, Some(self.user.address().into()))
            .call()
            .await
            .unwrap();
    }

    pub async fn get_domain_resolver(&self, domain: &str) -> Option<ContractId> {
        self.registry_contract
            .methods()
            .get_resolver(domain.to_string())
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value
    }

    pub async fn get_domain_expiration(&self, domain: &str) -> Option<u64> {
        self.registry_contract
            .methods()
            .get_expiration(domain.to_string())
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value
    }

    pub async fn get_domain_price(
        &self,
        domain: &str,
        years: u64,
        asset: &AssetId,
    ) -> u64 {
        self.registrar_contract
            .methods()
            .domain_price(domain.to_string(), years, *asset)
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value
    }

    pub async fn remove_fee_asset(
        &self,
        asset: &AssetId,
    ) {
        self.registrar_contract
            .methods()
            .remove_fee_asset(*asset)
            .call()
            .await
            .unwrap()
            .value
    }

    pub async fn _mint_domain(
        &self,
        domain: &str,
        years: u64,
        fee_to_transfer: u64,
        asset: Option<AssetId>,
    ) -> Result<AssetId> {
        let tx_policies = TxPolicies::default()
            .with_script_gas_limit(1_000_000);

        self.registrar_contract
            .clone()
            .with_account(self.user.clone())
            .methods()
            .mint_domain(domain.to_string(), years)
            .with_variable_output_policy(VariableOutputPolicy::Exactly(1))
            .with_tx_policies(tx_policies)
            .call_params(
                CallParameters::default()
                    .with_amount(fee_to_transfer)
                    .with_asset_id(asset.unwrap_or(BASE_ASSET_ID)),
            )
            .unwrap()
            .with_contracts(&[&self.registry_contract])
            .call()
            .await
            .map(|response| response.value)
    }

    pub async fn resolve_domain(&self, domain: &str) -> Option<Identity> {
        let resolver: ContractId = self
            .registry_contract
            .methods()
            .get_resolver(domain.to_string())
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value
            .unwrap();
        assert_eq!(resolver, self.resolver_contract.id().clone().into());
        let asset_id: AssetId = self
            .registry_contract
            .methods()
            .get_domain_asset_id(domain.to_string())
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value;
        let resolved: Option<Identity> = self
            .resolver_contract
            .methods()
            .resolve(asset_id)
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value;
        resolved
    }

    pub async fn reverse_resolve_domain(&self, identity: Identity) -> Option<AssetId> {
        let resolved: Option<AssetId> = self
            .registry_contract
            .methods()
            .resolve_to_primary_domain(identity)
            .with_contracts(&[&self.resolver_contract])
            .call()
            .await
            .unwrap()
            .value;
        resolved
    }

    pub async fn set_resolution(&self, domain: &str, to: Option<Identity>) {
        let asset_id: AssetId = self
            .registry_contract
            .methods()
            .get_domain_asset_id(domain.to_string())
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value;
        self.resolver_contract
            .clone()
            .with_account(self.user.clone())
            .methods()
            .set(asset_id, to)
            .with_contracts(&[&self.registry_contract])
            .add_custom_asset(asset_id, 1, Some(self.user.address().into()))
            .call()
            .await
            .unwrap();
    }

    pub async fn set_primary(&self, domain: &str) {
        let asset_id: AssetId = self
            .registry_contract
            .methods()
            .get_domain_asset_id(domain.to_string())
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value;
        self.registry_contract
            .clone()
            .with_account(self.user.clone())
            .methods()
            .set_primary(asset_id)
            .with_contracts(&[&self.resolver_contract])
            .call()
            .await
            .unwrap();
    }

    pub async fn withdraw_funds(&self) {
        self.registrar_contract
            .methods()
            .withdraw_funds()
            .with_variable_output_policy(VariableOutputPolicy::Exactly(1))
            .call()
            .await
            .unwrap();
    }

    pub async fn get_total_assets(&self) -> u64 {
        self.registry_contract
            .methods()
            .total_assets()
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value
    }

    pub async fn transfer(&self, owner: &WalletUnlocked, domain: &str, to: &Bech32Address) {
        let asset_id: AssetId = self.get_domain_asset_id(domain).await;
        owner
            .transfer(to, 1, asset_id, TxPolicies::default())
            .await
            .unwrap();

        // Send to Fuelet indexer
        let url = format!(
            "https://prod.api.fuelet.app/testnet/nft/73/mint/{}?domain={}&asset=0x{}",
            to.to_string(),
            domain,
            asset_id.to_string()
        );
        reqwest::Client::new().post(url).send().await.unwrap();
    }

    pub async fn set_fees(
        &self,
        asset: &AssetId,
        three_letter_fee: u64,
        four_letter_fee: u64,
        long_domain_fee: u64,
    ) {
        self.registrar_contract
            .methods()
            .set_fees(*asset, three_letter_fee, four_letter_fee, long_domain_fee)
            .call()
            .await
            .unwrap();
    }

    pub async fn set_grace_period(&self, duration: u64) {
        self.registrar_contract
            .methods()
            .set_grace_period(duration)
            .call()
            .await
            .unwrap();
    }

    pub async fn set_grace_period_as_user(&self, duration: u64) {
        self.registrar_contract
            .clone()
            .with_account(self.user.clone())
            .methods()
            .set_grace_period(duration)
            .call()
            .await
            .unwrap();
    }

    pub async fn get_grace_period(&self) -> u64 {
        self.registrar_contract
            .methods()
            .get_grace_period()
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value
    }

    pub async fn renew_domain(
        &self,
        domain: &str,
        years: u64,
        fee_to_transfer: u64,
    ) {
        self.registrar_contract
            .clone()
            .with_account(self.user.clone())
            .methods()
            .renew_domain(domain.to_string(), years)
            .call_params(
                CallParameters::default()
                    .with_amount(fee_to_transfer)
                    .with_asset_id(BASE_ASSET_ID),
            )
            .unwrap()
            .with_contracts(&[&self.registry_contract])
            .call()
            .await
            .unwrap();
    }

    pub async fn is_domain_active(&self, asset_id: AssetId) -> bool {
        self.registry_contract
            .methods()
            .is_domain_active(asset_id)
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value
    }

    pub async fn skip_n_days(&self, days: u32, for_deployer: bool) {
        let provider = if for_deployer {
            self.deployer.try_provider().unwrap()
        } else {
            self.user.try_provider().unwrap()
        };
        let block_timestamp = provider.latest_block_time().await.unwrap().unwrap();
        provider.produce_blocks(1, Some(block_timestamp + Duration::days(days as i64))).await.unwrap();
    }

    pub async fn get_timestamp(&self) -> i64 {
        self.user.try_provider().unwrap().latest_block_time().await.unwrap().unwrap().timestamp()
    }
}

pub async fn setup(on_chain: bool) -> Fixture {
    let (deployer, user) = get_wallets(on_chain).await;

    let registry_instance = get_registry_contract_instance(&deployer).await;
    let resolver_instance = get_resolver_contract_instance(&user, &registry_instance).await;
    let (registrar_instance, high_level_domain_asset) =
        get_registrar_contract_instance(&deployer, &registry_instance, &resolver_instance).await;

    Fixture {
        deployer: deployer.clone(),
        user: user.clone(),
        registry_contract: registry_instance,
        resolver_contract: resolver_instance,
        registrar_contract: registrar_instance,
        high_level_domain_asset,
    }
}

pub async fn connect_to_deployed_contracts() -> Fixture {
    let (deployer, user) = get_wallets(true).await;

    let registry_instance = Registry::new(
        ContractId::from_str(REGISTRY_DEPLOYED_CONTRACT_ID).unwrap(),
        deployer.clone(),
    );
    let resolver_instance = Resolver::new(
        ContractId::from_str(RESOLVER_DEPLOYED_CONTRACT_ID).unwrap(),
        user.clone(),
    );
    let registrar_instance = Registrar::new(
        ContractId::from_str(REGISTRAR_DEPLOYED_CONTRACT_ID).unwrap(),
        user.clone(),
    );

    Fixture {
        deployer: deployer.clone(),
        user: user.clone(),
        registry_contract: registry_instance,
        resolver_contract: resolver_instance,
        registrar_contract: registrar_instance,
        high_level_domain_asset: AssetId::default(),
    }
}

fn print_contract_id(contract_name: &str, contract_id: ContractId) {
    println!("{} contract deployed at: 0x{}", contract_name, contract_id);
}
