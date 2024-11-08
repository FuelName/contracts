contract;

mod string_util;
mod errors;

use ::errors::{AssetError, ValidationError, UnexpectedError, MintError, OwnershipError, RenewalError, ResolutionError};
use shared::{BaseDomainResolver, DomainRegistry, is_asset_owner};
use standards::src20::SRC20;
use standards::src7::{Metadata, SRC7};
use sway_libs::asset::{
    base::{
        _name,
        _symbol,
        _total_assets,
        _total_supply,
    },
    metadata::*,
};
use std::{hash::{Hash, sha256}, storage::storage_string::*, string::String, asset::mint_to, block::timestamp, inputs::{
        Input,
        input_asset_id,
        input_count,
        input_coin_owner,
        input_type,
    }};
use sway_libs::ownership::*;

const EXPIRATION_KEY: str[10] = __to_str_array("expiration");
const DOMAIN_NAME_KEY: str[11] = __to_str_array("domain_name");
const GRACE_PERIOD_KEY: str[12] = __to_str_array("grace_period");
const RESOLVER_KEY: str[8] = __to_str_array("resolver");

storage {
    total_assets: u64 = 0,
    metadata: StorageMetadata = StorageMetadata {},
    asset_genesis: StorageMap<b256, u64> = StorageMap {},
    primary_domains: StorageMap<Identity, AssetId> = StorageMap {},
}

impl SRC20 for Contract {
    #[storage(read)]
    fn total_assets() -> u64 {
        _total_assets(storage.total_assets)
    }

    #[storage(read)]
    fn total_supply(asset: AssetId) -> Option<u64> {
        if asset_exists(asset) {
            Some(1)
        } else {
            None 
        }
    }

    #[storage(read)]
    fn name(asset: AssetId) -> Option<String> {
        if asset_exists(asset) {
            Some(String::from_ascii_str("Fuelname"))
        } else {
            None
        }
    }
    
    #[storage(read)]
    fn symbol(asset: AssetId) -> Option<String> {
        if asset_exists(asset) {
            Some(String::from_ascii_str("FNS"))
        } else {
            None
        }
    }
    
    #[storage(read)]
    fn decimals(asset: AssetId) -> Option<u8> {
        if asset_exists(asset) {
            Some(0u8)
        } else {
            None 
        }
    }
}

impl SRC7 for Contract {
    #[storage(read)]
    fn metadata(asset: AssetId, key: String) -> Option<Metadata> {
        match get_domain_name(asset) {
            Some(domain) => {
                if key == String::from_ascii_str("tokenURI") {
                    return Some(Metadata::String(string_util::build_token_uri(domain)));
                }
                return None;
            },
            None => None,
        }
    }
}

#[storage(read)]
fn domain_to_asset_id(domain: String) -> (SubId, AssetId) {
    let last_gen = get_domain_gen(domain);
    let sub_id = sha256(string_util::build_domain_hash_base(domain, last_gen));
    let domain_id = AssetId::new(ContractId::this(), sub_id);
    (sub_id, domain_id)
}

#[storage(read)]
fn get_domain_name(asset: AssetId) -> Option<String> {
    match storage.metadata.get(asset, String::from_ascii_str(from_str_array(DOMAIN_NAME_KEY))) {
        Some(Metadata::String(s)) => Some(s),
        _ => None,
    }
}

#[storage(read)]
fn asset_exists(asset: AssetId) -> bool {
    match get_domain_name(asset) {
        Some(_) => (),
        None => return false,
    }
    let block_tai_timestamp = timestamp();
    let exp = match storage.metadata.get(asset, String::from_ascii_str(from_str_array(EXPIRATION_KEY))) {
        Some(Metadata::Int(exp)) => exp,
        _ => return true,
    };
    let grace_period = match storage.metadata.get(asset, String::from_ascii_str(from_str_array(GRACE_PERIOD_KEY))) {
        Some(Metadata::Int(gp)) => gp,
        _ => 0,
    };   
    exp + grace_period > block_tai_timestamp
}

#[storage(read)]
fn is_asset_active(asset: AssetId) -> bool {
    let domain_was_registered = get_domain_name(asset).is_some();
    match storage.metadata.get(asset, String::from_ascii_str(from_str_array(EXPIRATION_KEY))) {
        Some(Metadata::Int(exp)) => exp > timestamp(),
        _ => domain_was_registered,
    }
}

// TODO: provide the metadata url - pass URI from Registrar
// TODO: https://docs.ens.domains/registry/eth#commit-reveal check if no attacks available on the mempool
#[storage(read, write)]
fn mint_token(recipient: Identity, full_name: String, expiration: Option<u64>, grace_period: Option<u64>, resolver: Option<ContractId>) -> AssetId {
    let (_, old_asset_id) = domain_to_asset_id(full_name);
    require(!asset_exists(old_asset_id), MintError::AssetAlreadyMinted);
    let gen = get_domain_gen(full_name);
    storage.asset_genesis.insert(sha256(full_name), gen + 1);
    let (sub_id, asset_id) = domain_to_asset_id(full_name);
    let total_assets = _total_assets(storage.total_assets);
    storage.total_assets.write(total_assets + 1);
    mint_to(recipient, sub_id, 1);
    set_token_metadata(asset_id, full_name, expiration, grace_period, resolver);
    asset_id
}

#[storage(read, write)]
fn set_token_metadata(asset: AssetId, full_name: String, expiration: Option<u64>,  grace_period: Option<u64>, resolver: Option<ContractId>) {
    _set_metadata(storage.metadata, asset, String::from_ascii_str(from_str_array(DOMAIN_NAME_KEY)), Metadata::String(full_name));
    match expiration {
        Some(exp) => _set_metadata(storage.metadata, asset, String::from_ascii_str(from_str_array(EXPIRATION_KEY)), Metadata::Int(exp)),
        None => {},
    }
    match grace_period {
        Some(gp) => _set_metadata(storage.metadata, asset, String::from_ascii_str(from_str_array(GRACE_PERIOD_KEY)), Metadata::Int(gp)),
        None => {},
    }
    match resolver {
        Some(rslvr) => _set_metadata(storage.metadata, asset, String::from_ascii_str(from_str_array(RESOLVER_KEY)), Metadata::B256(rslvr.into())),
        None => {},
    }
}

#[storage(read)]
fn get_domain_gen(domain: String) -> u64 {
    let sha_domain = sha256(domain);
    storage.asset_genesis.get(sha_domain).try_read().unwrap_or(0)
}

fn validate_domain_name(domain: String) {
    require(string_util::domain_is_allowed(domain), ValidationError::InvalidDomainName);
}

fn validate_domain_name_part(domain_part: String) {
    require(string_util::domain_part_is_allowed(domain_part), ValidationError::InvalidDomainName);
}

#[storage(read)]
fn get_expiration_for_subdomain(asset: AssetId, expiration: Option<u64>) -> u64 {
    let parent_expiration = storage.metadata.get(asset, String::from_ascii_str(from_str_array(EXPIRATION_KEY)));
    match (parent_expiration, expiration) {
        (Some(Metadata::Int(parent)), Some(child)) => if parent > child { child } else { parent },
        (Some(Metadata::Int(parent)), None) => parent,
        (None, Some(child)) => child,
        (None, None) => {
            require(false, ValidationError::ExpirationNotSet);
            0
        }
        _ => {
            require(false, UnexpectedError::Unexpected);
            0
        }
    }
}

#[storage(read)]
fn check_parent_ownership_and_build_full_name(name: String, parent: String) -> String {
    let (_, parent_domain_asset) = domain_to_asset_id(parent);
    require(is_asset_owner(parent_domain_asset), OwnershipError::NotDomainOwner);
    string_util::build_domain_name(name, parent)
}

#[storage(read)]
fn get_resolved_address(asset: AssetId) -> Option<Identity> {
    if !is_asset_active(asset) {
        return None;
    }
    let resolver = match get_resolver_for_asset(asset) {
        Some(r) => r,
        None => {
            return None;
        }
    };
    let resolver_contract = abi(BaseDomainResolver, resolver.into()); 
    resolver_contract.resolve(asset)
} 

#[storage(read)]
fn get_resolver_for_asset(asset_id: AssetId) -> Option<ContractId> {
    // TODO: maybe we need to get resolver even if the domain is expired. To show legacy data
    if !asset_exists(asset_id) {
        return None;
    }
    match storage.metadata.get(asset_id, String::from_ascii_str(from_str_array(RESOLVER_KEY))) {
        Some(Metadata::B256(resolver)) => Some(ContractId::from(resolver)),
        _ => None,
    }
}


impl DomainRegistry for Contract {
    #[storage(read, write)]
    fn initialize() {
        let sender = msg_sender().unwrap();
        initialize_ownership(sender);
    }

    #[storage(read, write)]
    fn register_high_level_domain(recipient: Identity, name: String) -> AssetId {
        only_owner();
        validate_domain_name_part(name);
        validate_domain_name(name);
        let minted_asset = mint_token(recipient, name, None, None, None);
        minted_asset
    }

    #[storage(read, write)]
    fn register_sub_domain(recipient: Identity, parent: String, name: String, expiration: Option<u64>, grace_period: Option<u64>, resolver: ContractId) -> AssetId {
        let full_domain_name = check_parent_ownership_and_build_full_name(name, parent);
        validate_domain_name_part(name);
        validate_domain_name(full_domain_name);
        let (_, parent_domain_asset) = domain_to_asset_id(parent);
        let sub_domain_expiration = get_expiration_for_subdomain(parent_domain_asset, expiration);
        let minted_asset = mint_token(recipient, full_domain_name, Some(sub_domain_expiration), grace_period, Some(resolver));
        minted_asset
    }

    #[storage(read, write)]
    fn renew_domain(name: String, parent: String, expiration: u64) {
        let full_domain_name = check_parent_ownership_and_build_full_name(name, parent);
        let (_, asset_id) = domain_to_asset_id(full_domain_name);
        // check that asset exists and not exceed expiration + grace period
        require(asset_exists(asset_id), RenewalError::NoActiveDomainForRenewal);
        match storage.metadata.get(asset_id, String::from_ascii_str(from_str_array(EXPIRATION_KEY))) {
            Some(Metadata::Int(exp)) => require(expiration > exp, RenewalError::InvalidExpirationValue),
            _ => (),
        };
        _set_metadata(storage.metadata, asset_id, String::from_ascii_str(from_str_array(EXPIRATION_KEY)), Metadata::Int(expiration));
    }

    #[storage(read)]
    fn domain_exists(asset_id: AssetId) -> bool {
        asset_exists(asset_id)
    }

    #[storage(read)]
    fn is_domain_active(asset_id: AssetId) -> bool {
        is_asset_active(asset_id)
    }

    #[storage(read)]
    fn get_domain_asset_id(domain: String) -> AssetId {
       let (_, asset) = domain_to_asset_id(domain);
       asset
    }

    #[storage(read)]
    fn get_domain_name(asset: AssetId) -> String {
        let domain_name_opt = get_domain_name(asset);
        require(domain_name_opt.is_some(), ValidationError::DomainNotPresent);
        domain_name_opt.unwrap()
    }

    #[storage(read, write)]
    fn set_resolver(domain: String, resolver: ContractId) {
        let (_, asset) = domain_to_asset_id(domain); 
        require(is_asset_owner(asset), OwnershipError::NotDomainOwner);
        require(asset_exists(asset), AssetError::AssetDoesNotExist);
        _set_metadata(storage.metadata, asset, String::from_ascii_str(from_str_array(RESOLVER_KEY)), Metadata::B256(resolver.into()));
    }

    #[storage(read, write)]
    fn set_primary(asset: AssetId) {
        let sender = msg_sender().unwrap(); 
        storage.primary_domains.insert(sender, asset);
        require(get_resolved_address(asset) == Some(sender), ResolutionError::CannotSetPrimaryForUnknownAddress);
    }

    #[storage(read)]
    fn resolve_to_primary_domain(identity: Identity) -> Option<AssetId> {
        match storage.primary_domains.get(identity).try_read() {
            Some(domain_asset) => {
                if get_resolved_address(domain_asset) == Some(identity) { 
                    Some(domain_asset) 
                } else {
                    None
                }
            },
            None => None,
        }
    }

    // TODO: stick to the same interface everywhere and use AssetId?
    #[storage(read)]
    fn get_resolver(domain: String) -> Option<ContractId> {
        let (_, asset) = domain_to_asset_id(domain);
        get_resolver_for_asset(asset)
    }

    #[storage(read)]
    fn get_expiration(domain: String) -> Option<u64> {
        let (_, asset) = domain_to_asset_id(domain);
        match storage.metadata.get(asset, String::from_ascii_str(from_str_array(EXPIRATION_KEY))) {
            Some(Metadata::Int(exp)) => Some(exp),
            _ => None,
        }
    }

    #[storage(read)]
    fn get_expiration_by_parent(name: String, parent: String) -> Option<u64> {
        let domain = string_util::build_domain_name(name, parent);
        let (_, asset) = domain_to_asset_id(domain);
        match storage.metadata.get(asset, String::from_ascii_str(from_str_array(EXPIRATION_KEY))) {
            Some(Metadata::Int(exp)) => Some(exp),
            _ => None,
        }
    }

    #[storage(read)]
    fn get_grace_period(domain: String) -> Option<u64> {
        let (_, asset) = domain_to_asset_id(domain);
        match storage.metadata.get(asset, String::from_ascii_str(from_str_array(GRACE_PERIOD_KEY))) {
            Some(Metadata::Int(grace_period)) => Some(grace_period),
            _ => None,
        }
    }

}

// Tests
#[test]
fn test_total_assets() {
    let src20_abi = abi(SRC20, CONTRACT_ID);
    assert(src20_abi.total_assets() == 0);
}
#[test]
fn test_all_params_are_none_for_absent_nft() {
    use std::constants::ZERO_B256;
    let src20_abi = abi(SRC20, CONTRACT_ID);
    let absent_asset_id = AssetId::new(ContractId::from(CONTRACT_ID), ZERO_B256);
    assert(src20_abi.total_supply(absent_asset_id).is_none());
    assert(src20_abi.name(absent_asset_id).is_none());
    assert(src20_abi.symbol(absent_asset_id).is_none());
    assert(src20_abi.decimals(absent_asset_id).is_none());
}
