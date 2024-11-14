library;

use std::{constants::ZERO_B256, context::balance_of, string::String, inputs::{Input, input_asset_id, input_count, input_coin_owner, input_type}};

// TODO: add events (?)
abi DomainRegistry {
    #[storage(read, write)]
    fn initialize();

    #[storage(read, write)]
    fn register_high_level_domain(recipient: Identity, name: String) -> AssetId;

    #[storage(read, write)]
    fn register_sub_domain(recipient: Identity, parent: String, name: String, expiration: Option<u64>, grace_period: Option<u64>, resolver: ContractId) -> AssetId;

    #[storage(read)]
    fn domain_exists(asset: AssetId) -> bool;

    #[storage(read)]
    fn is_domain_active(asset: AssetId) -> bool;

    #[storage(read)]
    fn get_domain_asset_id(domain: String) -> AssetId;

    #[storage(read)]
    fn get_domain_name(asset: AssetId) -> String;
    
    #[storage(read, write)]
    fn set_resolver(domain: String, resolver: ContractId);

    #[storage(read)]
    fn get_resolver(domain: String) -> Option<ContractId>;

    #[storage(read)]
    fn get_expiration(domain: String) -> Option<u64>;

    #[storage(read)]
    fn get_expiration_by_parent(name: String, parent: String) -> Option<u64>;

    #[storage(read)]
    fn get_grace_period(domain: String) -> Option<u64>;

    #[storage(read, write)]
    fn renew_domain(name: String, parent: String, expiration: u64);

    #[storage(read, write)]
    fn set_primary(asset: AssetId);

    #[storage(read)]
    fn resolve_to_primary_domain(identity: Identity) -> Option<AssetId>;

}

/// Contains functions required for any resolver
abi BaseDomainResolver {
    #[storage(read)]
    fn resolve(asset: AssetId) -> Option<Identity>;
}

abi SimpleDomainResolver {
    #[storage(read, write)]
    fn set(asset: AssetId, resolve_to: Option<Identity>);
}

abi DomainRegistrar {
    #[storage(read, write)]
    fn initialize();

    #[storage(read)]
    fn domain_price(domain: String, years: u64, asset: AssetId) -> u64;

    #[payable]
    #[storage(read)]
    fn mint_domain(domain: String, years: u64) -> AssetId;

    #[payable]
    #[storage(read)]
    fn renew_domain(domain: String, years: u64);

    #[storage(read, write)]
    fn set_fees(asset: AssetId, three_letter_fee: u64, four_letter_fee: u64, long_domain_fee: u64);

    #[storage(read, write)]
    fn set_grace_period(grace_period: u64);

    #[storage(read)]
    fn get_grace_period() -> u64;

    #[storage(read)]
    fn withdraw_funds();

    #[storage(write)]
    fn remove_fee_asset(asset: AssetId);
}

// TODO: make sure the logic is correct and cannot be frauded
//  Can someone add an input to a transaction without owning it?
//  See https://forum.fuel.network/t/does-coin-input-in-a-transaction-guarantee-that-it-was-added-by-the-input-owner/4363
// TODO: think if we need to check if the asset was minted by the registry?
pub fn is_asset_owner(asset_id: AssetId) -> bool {
    match msg_sender() {
        Ok(Identity::ContractId(contract_id)) => {
            if balance_of(contract_id, asset_id) == 1 {
                return true;
            };
        },
        _ => (),
    };
    let num_inputs = input_count().as_u64();
    let mut i = 0;
    while i < num_inputs {
        match input_type(i) {
            Some(Input::Coin) => {
                let asset = input_asset_id(i).unwrap();
                if asset == asset_id {
                    return true;
                }
            },
            _ => {},
        }
        i = i + 1;
    }
    false
}
