contract;

mod errors;

use ::errors::{ExpirationError, ResolveError, OwnershipError};
use shared::{BaseDomainResolver, SimpleDomainResolver, is_asset_owner};
use std::{constants::ZERO_B256, hash::Hash};
use shared::DomainRegistry;

struct SetAddressEvent {
    asset: AssetId,
    identity: Option<Identity>,
}

configurable {
    REGISTRY_CONTRACT_ID: ContractId = ContractId::from(ZERO_B256),
}

storage {
    resolved_addresses: StorageMap<AssetId, Identity> = StorageMap {},
}

#[storage(read, write)]
fn set_resolved_address(asset: AssetId, resolve_to: Option<Identity>) {
    match resolve_to {
        Some(identity) => {
            storage.resolved_addresses.insert(asset, identity);
        },
        None => {
            let _ = storage.resolved_addresses.remove(asset);
        },
    }
}

fn is_domain_active(asset: AssetId) -> bool {
    let registry_contract = abi(DomainRegistry, REGISTRY_CONTRACT_ID.into());
    registry_contract.is_domain_active(asset)
}

impl BaseDomainResolver for Contract {
    #[storage(read)]
    fn resolve(asset: AssetId) -> Option<Identity> {
        // Expired domains should be handled by the registry.
        // We can't enforce all resolvers to handle this
        storage.resolved_addresses.get(asset).try_read()
    }
}

impl SimpleDomainResolver for Contract {
    #[storage(read, write)]
    fn set(asset: AssetId, resolve_to: Option<Identity>) {
        require(is_asset_owner(asset), OwnershipError::NotDomainOwner);
        set_resolved_address(asset, resolve_to);
        require(is_domain_active(asset), ExpirationError::ExpiredDomain);
        log(SetAddressEvent { asset, identity: resolve_to });
    }
}
