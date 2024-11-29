use fuels::{accounts::wallet::WalletUnlocked, prelude::*};
use std::collections::HashMap;

use crate::deployer::{ContractType, DeployResult};
use crate::fixture::Caller::{Deployer, User};
use chrono::Duration;
use fuelname_sdk::interface::{DomainId, Fee, Metadata, Registrar, RegistrarReadonlySdk, RegistrarSdk, Registry, RegistryReadonlySdk, RegistrySdk, Resolver};
use fuelname_sdk::interface::{FuelnameContracts, Sdk};
use fuels::types::Identity;

pub struct Fixture {
    pub deployer: WalletUnlocked,
    pub user: WalletUnlocked,
    pub registry_contract: Registry<WalletUnlocked>,
    pub resolver_contract: Resolver<WalletUnlocked>,
    pub registrar_contract: Registrar<WalletUnlocked>,
    pub contracts: HashMap<ContractType, DeployResult>,
}

impl Fixture {
    fn registry(&self) -> DeployResult {
        self.contracts.get(&ContractType::Registry).unwrap().clone()
    }

    fn registrar(&self) -> DeployResult {
        self.contracts.get(&ContractType::Registrar).unwrap().clone()
    }

    fn resolver(&self) -> DeployResult {
        self.contracts.get(&ContractType::Resolver).unwrap().clone()
    }

    pub async fn mint_domain(
        &self,
        domain: &str,
        years: u64,
        fee_to_transfer: u64,
    ) -> Result<AssetId> {
        self._mint_domain(domain, years, fee_to_transfer, None).await
    }

    pub async fn domain_exists(&self, asset_id: AssetId) -> bool {
        self.sdk(Deployer).await.domain_exists(DomainId::Asset(asset_id)).await.unwrap().value
    }

    pub async fn get_domain_asset_id(&self, domain: &str) -> AssetId {
        self.sdk(Deployer).await.get_domain_asset(domain).await.unwrap().value
    }

    pub async fn get_domain_name(&self, asset: AssetId) -> String {
        self.sdk(Deployer).await.get_domain_name(asset).await.unwrap().value
    }

    pub async fn get_token_uri(&self, asset: AssetId) -> Option<Metadata> {
        let metadata: Option<Metadata> = self
            .registry_contract
            .methods()
            .metadata(asset, "tokenURI".to_string())
            .with_contract_ids(&[self.registry().target_id.into()])
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
            .with_contract_ids(&[self.registry().target_id.into()])
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value;
        self.registry_contract
            .clone()
            .with_account(self.user.clone())
            .methods()
            .set_resolver(domain.to_string(), resolver)
            .with_contract_ids(&[self.registry().target_id.into()])
            .add_custom_asset(asset_id, 1, Some(self.user.address().into()))
            .call()
            .await
            .unwrap();
    }

    pub async fn get_domain_resolver(&self, domain: &str) -> Option<ContractId> {
        self.sdk(Deployer).await.get_domain_resolver_id(domain).await.unwrap().value
    }

    pub async fn get_domain_expiration(&self, domain: &str) -> Option<u64> {
        self.sdk(Deployer).await.get_domain_expiration(domain).await.unwrap().value
    }

    pub async fn get_domain_price(
        &self,
        domain: &str,
        years: u64,
        asset: &AssetId,
    ) -> u64 {
        self.sdk(User).await.get_domain_price(domain, years, asset).await.unwrap().value
    }

    pub async fn remove_fee_asset(
        &self,
        asset: &AssetId,
    ) {
        self.registrar_contract
            .methods()
            .remove_fee_asset(*asset)
            .with_contract_ids(&[self.registrar().target_id.into()])
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
        self.sdk(User).await
            .mint_domain(
                domain,
                years,
                Fee {
                    asset: asset.unwrap_or(AssetId::BASE),
                    amount: fee_to_transfer,
                },
                None,
            )
            .await
            .map(|response| response.value)
    }

    pub async fn resolve_domain(&self, domain: &str) -> Option<Identity> {
        self.sdk(Deployer).await.resolve_domain_to_address(domain).await.unwrap().value
    }

    pub async fn reverse_resolve_domain(&self, identity: Identity) -> Option<AssetId> {
        let domain = self.sdk(Deployer).await.get_primary_domain(identity).await.unwrap().value;
        match domain {
            Some(domain) => {
                let asset = self.get_domain_asset_id(&domain).await;
                Some(asset)
            }
            None => None,
        }
    }

    pub async fn set_resolution(&self, domain: &str, to: Option<Identity>) {
        self.sdk(User).await.set_address(domain, to, None).await.unwrap();
    }

    pub async fn set_primary(&self, domain: &str) {
        self.sdk(User).await.set_primary_domain(domain, None).await.unwrap();
    }

    pub async fn withdraw_funds(&self, asset_id: &AssetId) {
        self.registrar_contract
            .methods()
            .withdraw_funds(*asset_id)
            .with_contract_ids(&[self.registrar().target_id.into()])
            .with_variable_output_policy(VariableOutputPolicy::Exactly(1))
            .call()
            .await
            .unwrap();
    }

    pub async fn get_total_assets(&self) -> u64 {
        self.registry_contract
            .methods()
            .total_assets()
            .with_contract_ids(&[self.registry().target_id.into()])
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
            .with_contract_ids(&[self.registrar().target_id.into()])
            .call()
            .await
            .unwrap();
    }

    pub async fn set_grace_period(&self, duration: u64) {
        self.registrar_contract
            .methods()
            .set_grace_period(duration)
            .with_contract_ids(&[self.registrar().target_id.into()])
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
            .with_contract_ids(&[self.registrar().target_id.into()])
            .call()
            .await
            .unwrap();
    }

    pub async fn get_grace_period(&self) -> u64 {
        self.registrar_contract
            .methods()
            .get_grace_period()
            .with_contract_ids(&[self.registrar().target_id.into()])
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
            .with_contract_ids(&[
                self.registrar().target_id.into(),
                self.registry().proxy_id.into(),
                self.registry().target_id.into(),
            ])
            .call_params(
                CallParameters::default()
                    .with_amount(fee_to_transfer)
                    .with_asset_id(AssetId::BASE),
            )
            .unwrap()
            .call()
            .await
            .unwrap();
    }

    pub async fn is_domain_active(&self, asset_id: AssetId) -> bool {
        self.registry_contract
            .methods()
            .is_domain_active(asset_id)
            .with_contract_ids(&[self.registry().target_id.into()])
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

    pub fn connect(
        deployer: WalletUnlocked,
        user: WalletUnlocked,
        contracts: HashMap<ContractType, DeployResult>,
    ) -> Fixture {
        let registry = contracts.get(&ContractType::Registry).unwrap();
        let resolver = contracts.get(&ContractType::Resolver).unwrap();
        let registrar = contracts.get(&ContractType::Registrar).unwrap();
        Fixture {
            deployer: deployer.clone(),
            user: user.clone(),
            registry_contract: Registry::new(registry.proxy_id, deployer.clone()),
            resolver_contract: Resolver::new(resolver.proxy_id, user.clone()),
            registrar_contract: Registrar::new(registrar.proxy_id, user.clone()),
            contracts,
        }
    }

    async fn sdk(&self, caller: Caller) -> impl Sdk {
        let registry = self.contracts.get(&ContractType::Registry).unwrap();
        let registrar = self.contracts.get(&ContractType::Registrar).unwrap();
        let wallet = match caller {
            Deployer => self.deployer.clone(),
            User => self.user.clone(),
        };
        FuelnameContracts::connect(
            wallet,
            Some(registrar.proxy_id),
            Some(registry.proxy_id),
        ).await.unwrap()
    }
}

enum Caller {
    Deployer,
    User,
}
