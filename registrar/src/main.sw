contract;

mod errors;

use ::errors::ValidationError;
use ::errors::GracePeriodError;
use ::errors::DomainRenewalError;

use shared::{DomainRegistrar, DomainRegistry};
use std::{constants::ZERO_B256, call_frames::{msg_asset_id}, string::String, outputs::{Output, output_type, output_count, output_amount, output_asset_id, output_asset_to}, block::timestamp, context::msg_amount, asset::transfer, context::this_balance};
use sway_libs::ownership::*;

configurable {
    REGISTRY_CONTRACT_ID: ContractId = ContractId::from(ZERO_B256),
    DEFAULT_RESOLVER_CONTRACT_ID: ContractId = ContractId::from(ZERO_B256),

    RESERVER_ADDRESS: b256 = 0xaebc5eac48e2d83bfaae60f9d674ac3f1e7f5dd51f1c102adfa04edda7be7e31,
}

const ONE_YEAR_SECONDS: u64 = 31622400; 
const MIN_GRACE_PERIOD_DURATION = 2592000; // 30 days
const ROOT_DOMAIN: str[4] = __to_str_array("fuel");

storage {
    three_letter_annual_fee: u64 = 50000000,
    four_letter_annual_fee: u64  = 10000000,
    long_domain_annual_fee: u64  = 1000000,
    grace_period_duration: u64 = MIN_GRACE_PERIOD_DURATION,
}

#[storage(read)]
fn get_domain_price(domain: String, years: u64) -> u64 {
    if msg_sender().unwrap() == Identity::Address(Address::from(RESERVER_ADDRESS)) {
        return 0;
    }
    let length = domain.as_bytes().len();
    require(length >= 3, ValidationError::InvalidDomainName); // TODO: duplication of string_utils. Move to a shared module
    require(years > 0 && years <= 3, ValidationError::InvalidPeriod);
    let annual_fee = if length == 3 {
        storage.three_letter_annual_fee.read()
    } else if length == 4 {
        storage.four_letter_annual_fee.read()
    } else {
        storage.long_domain_annual_fee.read()
    };
    annual_fee * years
}

#[storage(read)]
fn check_domain_payment(name: String, years: u64) {
    let price = get_domain_price(name, years);
    let paid = get_transferred_eth_amount();
    require(price == paid, ValidationError::WrongFeeAmount);
}

fn get_transferred_eth_amount() -> u64 {
    // TODO: replace base asset with eth asset id? In case base changes
    if msg_asset_id() == AssetId::base() {
        msg_amount()
    } else {
        0
    }
}

fn years_from_now_ts(years: u64) -> u64 {
    let block_tai_timestamp = timestamp();
    let ttl = ONE_YEAR_SECONDS * years;
    block_tai_timestamp + ttl
}

impl DomainRegistrar for Contract {
    #[storage(read, write)]
    fn initialize() {
        let sender = msg_sender().unwrap();
        initialize_ownership(sender);
    }

    #[storage(read)]
    fn domain_price(domain: String, years: u64) -> u64 {
        get_domain_price(domain, years)
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
    fn set_fees(three_letter_fee: u64, four_letter_fee: u64, long_domain_fee: u64) {
        only_owner();
        storage.three_letter_annual_fee.write(three_letter_fee);
        storage.four_letter_annual_fee.write(four_letter_fee);
        storage.long_domain_annual_fee.write(long_domain_fee);
    }

    #[storage(read, write)]
    fn set_grace_period(grace_period_duration: u64) {
        only_owner();
        require(grace_period_duration >= MIN_GRACE_PERIOD_DURATION, GracePeriodError::InvalidGracePeriodDuration);
        storage.grace_period_duration.write(grace_period_duration);
    }

    #[storage(read)]
    fn get_grace_period() -> u64 {
        storage.grace_period_duration.read()
    } 

    // TODO: allow the NFT withdrawal
    #[storage(read)]
    fn withdraw_funds() {
        only_owner();
        let sender = msg_sender().unwrap();
        let balance = this_balance(AssetId::base());
        transfer(sender, AssetId::base(), balance);
    }
}
