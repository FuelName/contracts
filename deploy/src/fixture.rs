use fuels::{accounts::wallet::WalletUnlocked, prelude::*};
use std::collections::HashMap;

use crate::deployer::{ContractType, DeployResult, Metadata, Registrar, Registry, Resolver};
use chrono::Duration;
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
        self.registry_contract
            .methods()
            .domain_exists(asset_id)
            .with_contract_ids(&[self.registry().target_id.into()])
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value
    }

    pub async fn get_domain_asset_id(&self, domain: &str) -> AssetId {
        self.registry_contract
            .methods()
            .get_domain_asset_id(domain.to_string())
            .with_contract_ids(&[self.registry().target_id.into()])
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value
    }

    pub async fn get_domain_name(&self, asset: AssetId) -> String {
        self.registry_contract
            .methods()
            .get_domain_name(asset)
            .with_contract_ids(&[self.registry().target_id.into()])
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
        self.registry_contract
            .methods()
            .get_resolver(domain.to_string())
            .with_contract_ids(&[self.registry().target_id.into()])
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value
    }

    pub async fn get_domain_expiration(&self, domain: &str) -> Option<u64> {
        self.registry_contract
            .methods()
            .get_expiration(domain.to_string())
            .with_contract_ids(&[self.registry().target_id.into()])
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
            .with_contract_ids(&[self.registrar().target_id.into()])
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
                    .with_asset_id(asset.unwrap_or(AssetId::BASE)),
            )
            .unwrap()
            .with_contract_ids(&[
                self.registrar().target_id.into(),
                self.registry().proxy_id.into(),
                self.registry().target_id.into()
            ])
            .call()
            .await
            .map(|response| response.value)
    }

    pub async fn resolve_domain(&self, domain: &str) -> Option<Identity> {
        let resolver: ContractId = self
            .registry_contract
            .methods()
            .get_resolver(domain.to_string())
            .with_contract_ids(&[self.registry().target_id.into()])
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
            .with_contract_ids(&[self.registry().target_id.into()])
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value;
        let resolved: Option<Identity> = self
            .resolver_contract
            .methods()
            .resolve(asset_id)
            .with_contract_ids(&[self.resolver().target_id.into()])
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
            .with_contract_ids(&[
                self.registry().target_id.into(),
                self.resolver().proxy_id.into(),
                self.resolver().target_id.into(),
            ])
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
            .with_contract_ids(&[
                self.registry().target_id.into(),
                self.resolver().proxy_id.into(),
                self.resolver().target_id.into(),
            ])
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value;
        self.resolver_contract
            .clone()
            .with_account(self.user.clone())
            .methods()
            .set(asset_id, to)
            .with_contract_ids(&[
                self.registry().proxy_id.into(),
                self.registry().target_id.into(),
                self.resolver().target_id.into()
            ])
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
            .with_contract_ids(&[self.registry().target_id.into()])
            .simulate(Execution::StateReadOnly)
            .await
            .unwrap()
            .value;
        self.registry_contract
            .clone()
            .with_account(self.user.clone())
            .methods()
            .set_primary(asset_id)
            .with_contract_ids(&[
                self.registry().target_id.into(),
                self.resolver().proxy_id.into(),
                self.resolver().target_id.into(),
            ])
            .call()
            .await
            .unwrap();
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
}
