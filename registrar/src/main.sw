contract;

mod errors;

use ::errors::ValidationError;
use ::errors::GracePeriodError;
use ::errors::DomainRenewalError;

use shared::{DomainRegistrar, DomainRegistry};
use std::{hash::Hash, constants::ZERO_B256, call_frames::{msg_asset_id}, string::String, outputs::{Output, output_type, output_count, output_amount, output_asset_id, output_asset_to}, block::timestamp, context::msg_amount, asset::transfer, context::this_balance};
use sway_libs::ownership::*;

struct SetFeesEvent {
    asset_id: AssetId,
    fees: Fees,
}

struct RemoveFeeAssetEvent {
    asset_id: AssetId,
}

struct SetGracePeriodEvent {
    duration: u64,
}

struct Fees {
    three_letter_annual_fee: u64,
    four_letter_annual_fee: u64,
    long_domain_annual_fee: u64,
}

configurable {
    REGISTRY_CONTRACT_ID: ContractId = ContractId::from(ZERO_B256),
    DEFAULT_RESOLVER_CONTRACT_ID: ContractId = ContractId::from(ZERO_B256),
    ETH_FEES: Fees = Fees {
        three_letter_annual_fee: 50000000,
        four_letter_annual_fee: 10000000,
        long_domain_annual_fee: 1000000,
    },
    RESERVER_ADDRESS: b256 = 0xaebc5eac48e2d83bfaae60f9d674ac3f1e7f5dd51f1c102adfa04edda7be7e31,
}

const ONE_YEAR_SECONDS: u64 = 31622400; 
const MIN_GRACE_PERIOD_DURATION = 2592000; // 30 days
const ROOT_DOMAIN: str[4] = __to_str_array("fuel");

storage {
    grace_period_duration: u64 = MIN_GRACE_PERIOD_DURATION,
    pricing: StorageMap<AssetId, Fees> = StorageMap {},
}

#[storage(read)]
fn get_domain_price(asset: AssetId, domain: String, years: u64) -> u64 {
    if msg_sender().unwrap() == Identity::Address(Address::from(RESERVER_ADDRESS)) {
        return 0;
    }
    let length = domain.as_bytes().len();
    require(length >= 3, ValidationError::InvalidDomainName); // TODO: duplication of string_utils. Move to a shared module
    require(years > 0 && years <= 3, ValidationError::InvalidPeriod);
    let fees = storage.pricing.get(asset).try_read();
    require(fees.is_some(), ValidationError::WrongFeeAsset);
    let fees = fees.unwrap();
    let annual_fee = if length == 3 {
        fees.three_letter_annual_fee
    } else if length == 4 {
        fees.four_letter_annual_fee
    } else {
        fees.long_domain_annual_fee
    };
    annual_fee * years
}

#[storage(read)]
fn check_domain_payment(name: String, years: u64) {
    let asset_id = msg_asset_id();
    let price = get_domain_price(asset_id, name, years);
    let paid = msg_amount();
    require(price == paid, ValidationError::WrongFeeAmount);
}

fn years_from_now_ts(years: u64) -> u64 {
    let block_tai_timestamp = timestamp();
    let ttl = ONE_YEAR_SECONDS * years;
    block_tai_timestamp + ttl
}

impl DomainRegistrar for Contract {
    #[storage(read, write)]
    fn initialize() -> Identity {
        let sender = msg_sender().unwrap();
        initialize_ownership(sender);
        storage.pricing.insert(AssetId::base(), ETH_FEES);
        // set grace period here to make it accessible through proxy
        storage.grace_period_duration.write(MIN_GRACE_PERIOD_DURATION);
        log(
            SetFeesEvent {
                asset_id: AssetId::base(),
                fees: ETH_FEES 
            }
        );
        sender
    }

    #[storage(read)]
    fn domain_price(domain: String, years: u64, asset: AssetId) -> u64 {
        get_domain_price(asset, domain, years)
    }

    #[payable]
    #[storage(read)]
    fn mint_domain(domain: String, years: u64) -> AssetId {
        let sender = msg_sender().unwrap();
        check_domain_payment(domain, years);
        let registry_contract = abi(DomainRegistry, REGISTRY_CONTRACT_ID.into());
        let expiration_ts = years_from_now_ts(years);
        let minted_asset = registry_contract.register_sub_domain(
            sender,
            String::from_ascii_str(from_str_array(ROOT_DOMAIN)),
            domain,
            Some(expiration_ts),
            Some(storage.grace_period_duration.read()),
            DEFAULT_RESOLVER_CONTRACT_ID
        );
        minted_asset
    }

    #[payable]
    #[storage(read)]
    fn renew_domain(name: String, years: u64) {
        check_domain_payment(name, years);
        let registry_contract = abi(DomainRegistry, REGISTRY_CONTRACT_ID.into());
        let current_expiration = match registry_contract.get_expiration_by_parent(name, String::from_ascii_str(from_str_array(ROOT_DOMAIN))) {
            Some(exp) => exp,
            None => {
                        require(false, DomainRenewalError::CanNotRenewRootDomain);
                        0
                    }
        };
        registry_contract.renew_domain(
            name,
            String::from_ascii_str(from_str_array(ROOT_DOMAIN)),
            current_expiration + (years * ONE_YEAR_SECONDS)
        );
    }

    #[storage(read, write)]
    fn set_fees(asset: AssetId, three_letter_fee: u64, four_letter_fee: u64, long_domain_fee: u64) {
        only_owner();
        let fees = Fees {
            three_letter_annual_fee: three_letter_fee,
            four_letter_annual_fee: four_letter_fee,
            long_domain_annual_fee: long_domain_fee,
        };
        storage.pricing.insert(asset, fees);
        log(
            SetFeesEvent {
                asset_id: asset,
                fees 
            }
        );
    }

    #[storage(read, write)]
    fn set_grace_period(grace_period_duration: u64) {
        only_owner();
        require(grace_period_duration >= MIN_GRACE_PERIOD_DURATION, GracePeriodError::InvalidGracePeriodDuration);
        storage.grace_period_duration.write(grace_period_duration);
        log(
            SetGracePeriodEvent {
                duration: grace_period_duration
            }
        );
    }

    #[storage(read)]
    fn get_grace_period() -> u64 {
        storage.grace_period_duration.read()
    } 

    #[storage(read)]
    fn withdraw_funds(asset_id: AssetId) {
        only_owner();
        let sender = msg_sender().unwrap();
        let balance = this_balance(asset_id);
        transfer(sender, asset_id, balance);
    }

    #[storage(write)]
    fn remove_fee_asset(asset: AssetId) {
        only_owner();
        let removed = storage.pricing.remove(asset);
        require(removed, ValidationError::WrongFeeAsset);
        log(
            RemoveFeeAssetEvent {
                asset_id: asset
            }
        );
    }
}
