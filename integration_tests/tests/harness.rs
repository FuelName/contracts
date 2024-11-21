use std::str::FromStr;
use fuels::prelude::*;
use fuels::types::Identity;
use deploy::deployer;
use deploy::deployer::{DeployParams, DeployTarget, LocalDeployParams};
use deploy::fixture::Fixture;

const THREE_LETTER_ANNUAL_DEFAULT_FEE: u64 = 50000000;
const FOUR_LETTER_ANNUAL_DEFAULT_FEE: u64 = 10000000;
const COMMON_ANNUAL_DEFAULT_FEE: u64 = 1000000;
const MIN_GRACE_PERIOD_DURATION: u64 = 2592000; // 30 days
const ONE_YEAR_SECONDS: u64 = 31622400;

const HIGH_LEVEL_DOMAIN: &str = "fuel";
const SUB_DOMAIN_PART_1: &str = "fuelname";
const SUB_DOMAIN_PART_2: &str = "fuelet";
const SUB_DOMAIN_1: &str = "fuelname.fuel";
const SUB_DOMAIN_2: &str = "fuelet.fuel";
const COMMON_DEFAULT_FEE: u64 = COMMON_ANNUAL_DEFAULT_FEE;
const THREE_LETTER_DEFAULT_FEE: u64 = THREE_LETTER_ANNUAL_DEFAULT_FEE;
const FOUR_LETTER_DEFAULT_FEE: u64 = FOUR_LETTER_ANNUAL_DEFAULT_FEE;
const BASE_ASSET_ID: AssetId = AssetId::BASE;
pub fn usdc_asset_id() -> AssetId {
    AssetId::from_str("0x286c479da40dc953bddc3bb4c453b608bba2e0ac483b077bd475174115395e6b").unwrap()
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

async fn setup() -> Fixture {
    let (deployer, user) = get_custom_wallets().await;
    let params: LocalDeployParams = LocalDeployParams {
        deployer_wallet: deployer,
        user_wallet: user,
        deploy_params: DeployParams::InitialDeploy,
    };
    deployer::deploy(DeployTarget::Local(params)).await
}

mod tests {
    use super::*;
    use rand::random;

    #[tokio::test]
    async fn test_high_level_domain() {
        let fixture = setup().await;

        let domain_asset_id = fixture.get_domain_asset_id(HIGH_LEVEL_DOMAIN).await;
        let domain_exists = fixture.domain_exists(domain_asset_id).await;
        let domain_resolver = fixture.get_domain_resolver(HIGH_LEVEL_DOMAIN).await;
        let domain_expiration = fixture.get_domain_expiration(HIGH_LEVEL_DOMAIN).await;
        let domain_name = fixture.get_domain_name(domain_asset_id).await;

        assert_eq!(domain_exists, true);
        assert_eq!(domain_asset_id.len(), 32); // just check that the function returns result
        assert_eq!(domain_resolver, None);
        assert_eq!(domain_expiration, None);
        assert_eq!(domain_name, HIGH_LEVEL_DOMAIN.to_string());
    }

    #[tokio::test]
    async fn test_mint_domain() {
        let fixture = setup().await;

        let domain_asset_id_before_minting = fixture.get_domain_asset_id(SUB_DOMAIN_1).await;
        let domain_exist_before_minting = fixture.domain_exists(domain_asset_id_before_minting).await;
        let domain_resolver_before_minting = fixture.get_domain_resolver(SUB_DOMAIN_1).await;
        let domain_expiration_before_minting = fixture.get_domain_expiration(SUB_DOMAIN_1).await;
        let user_balance_before_minting = fixture.user.get_asset_balance(&domain_asset_id_before_minting).await.unwrap();

        assert_eq!(domain_exist_before_minting, false);
        assert_eq!(domain_resolver_before_minting, None);
        assert_eq!(domain_expiration_before_minting, None);
        assert_eq!(user_balance_before_minting, 0);

        let asset = fixture.mint_domain(SUB_DOMAIN_PART_1, 2, COMMON_DEFAULT_FEE * 2).await.unwrap();

        let domain_asset_id = fixture.get_domain_asset_id(SUB_DOMAIN_1).await;
        let domain_exists = fixture.domain_exists(domain_asset_id).await;
        let domain_resolver = fixture.get_domain_resolver(SUB_DOMAIN_1).await;
        let domain_expiration = fixture.get_domain_expiration(SUB_DOMAIN_1).await;
        let user_balance = fixture.user.get_asset_balance(&domain_asset_id).await.unwrap();
        let domain_name = fixture.get_domain_name(asset).await;

        assert_eq!(domain_exists, true);
        assert_eq!(domain_resolver, Some(fixture.resolver_contract.contract_id().clone().into()));
        assert_eq!(domain_expiration.is_some(), true);
        assert_eq!(user_balance, 1);
        assert_eq!(domain_name, SUB_DOMAIN_1.to_string());
    }

    #[tokio::test]
    async fn test_fail_to_mint_the_same_domain_twice() {
        let fixture = setup().await;
        fixture.mint_domain(SUB_DOMAIN_PART_1, 2, COMMON_DEFAULT_FEE * 2).await.unwrap();

        let second_mint_result = fixture.mint_domain(SUB_DOMAIN_PART_1, 2, COMMON_DEFAULT_FEE * 2).await;
        assert_eq!(second_mint_result.is_err(), true);

        let domain_asset_id = fixture.get_domain_asset_id(SUB_DOMAIN_1).await;
        let user_balance = fixture.user.get_asset_balance(&domain_asset_id).await.unwrap();
        assert_eq!(user_balance, 1);
    }

    #[tokio::test]
    async fn test_fail_to_mint_with_inappropriate_fee() {
        let fixture = setup().await;
        let result = fixture.mint_domain(SUB_DOMAIN_PART_1, 2, COMMON_DEFAULT_FEE).await;
        assert_eq!(result.is_err(), true);
    }

    #[tokio::test]
    async fn test_mint_two_domains() {
        let fixture = setup().await;
        fixture.mint_domain(SUB_DOMAIN_PART_1, 2, COMMON_DEFAULT_FEE * 2).await.unwrap();
        fixture.mint_domain(SUB_DOMAIN_PART_2, 1, COMMON_DEFAULT_FEE).await.unwrap();

        let domain_asset_id_1 = fixture.get_domain_asset_id(SUB_DOMAIN_1).await;
        let domain_asset_id_2 = fixture.get_domain_asset_id(SUB_DOMAIN_2).await;

        let user_balance_1 = fixture.user.get_asset_balance(&domain_asset_id_1).await.unwrap();
        let user_balance_2 = fixture.user.get_asset_balance(&domain_asset_id_2).await.unwrap();
        assert_eq!(user_balance_1, 1);
        assert_eq!(user_balance_2, 1);
    }

    #[tokio::test]
    async fn test_mint_domains_of_different_length() {
        let fixture = setup().await;
        for len in 3..60 {
            let fee = if len == 3 { THREE_LETTER_DEFAULT_FEE } else if len == 4 { FOUR_LETTER_DEFAULT_FEE } else { COMMON_DEFAULT_FEE };
            let domain = "a".repeat(len);
            let full = format!("{}.fuel", domain);
            println!("Testing len {}: '{}'", full.len(), full);
            let minted_asset = fixture.mint_domain(&domain, 1, fee).await.unwrap();
            let balance = fixture.user.get_asset_balance(&minted_asset).await.unwrap();
            assert_eq!(balance, 1);
        }
    }

    #[tokio::test]
    async fn test_mint_domains_of_invalid_length() {
        let fixture = setup().await;
        for invalid_len in [1, 2, 60, 61].iter() {
            let domain = "a".repeat(*invalid_len);
            let full = format!("{}.fuel", domain);
            println!("Testing len {}: '{}'", full.len(), full);
            let result = fixture.mint_domain(&domain, 1, COMMON_DEFAULT_FEE).await;
            assert_eq!(result.is_err(), true);
        }
    }

    #[tokio::test]
    async fn test_mint_domains_with_invalid_symbols() {
        let fixture = setup().await;
        for invalid_domain in ["абвгд", "abc_def", "!1!!1", "𡨸漢𡨸漢"].iter() {
            let full = format!("{}.fuel", invalid_domain);
            println!("Testing invalid symbols of '{}'", full);
            let result = fixture.mint_domain(&invalid_domain, 1, COMMON_DEFAULT_FEE).await;
            assert_eq!(result.is_err(), true);
        }
    }

    #[tokio::test]
    async fn test_domain_resolution() {
        let fixture = setup().await;
        let user_identity = Identity::Address(fixture.user.address().into());
        let minted_asset = fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
        let before_set = fixture.resolve_domain(SUB_DOMAIN_1).await;
        let before_set_reverse = fixture.reverse_resolve_domain(user_identity.clone()).await;
        assert_eq!(before_set, None);
        assert_eq!(before_set_reverse, None);

        let balance = fixture.user.get_asset_balance(&minted_asset).await.unwrap();
        assert_eq!(balance, 1);
        fixture.set_resolution(SUB_DOMAIN_1, Some(user_identity.clone())).await;
        fixture.set_primary(SUB_DOMAIN_1).await;

        let after_set = fixture.resolve_domain(SUB_DOMAIN_1).await;
        let after_set_reverse = fixture.reverse_resolve_domain(user_identity.clone()).await;
        assert_eq!(after_set, Some(user_identity.clone()));
        assert_eq!(after_set_reverse, Some(minted_asset));
    }

    #[tokio::test]
    async fn test_primary_domain_resolution() {
        let fixture = setup().await;
        let user_identity = Identity::Address(fixture.user.address().into());
        let minted_asset_1 = fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
        let minted_asset_2 = fixture.mint_domain(SUB_DOMAIN_PART_2, 1, COMMON_DEFAULT_FEE).await.unwrap();

        fixture.set_resolution(SUB_DOMAIN_1, Some(user_identity.clone())).await;
        fixture.set_resolution(SUB_DOMAIN_2, Some(user_identity.clone())).await;

        assert_eq!(fixture.resolve_domain(SUB_DOMAIN_1).await, Some(user_identity.clone()));
        assert_eq!(fixture.resolve_domain(SUB_DOMAIN_2).await, Some(user_identity.clone()));
        assert_eq!(fixture.reverse_resolve_domain(user_identity.clone()).await, None);

        fixture.set_resolution(SUB_DOMAIN_2, Some(user_identity.clone())).await;
        fixture.set_primary(SUB_DOMAIN_2).await;

        assert_eq!(fixture.resolve_domain(SUB_DOMAIN_1).await, Some(user_identity.clone()));
        assert_eq!(fixture.resolve_domain(SUB_DOMAIN_2).await, Some(user_identity.clone()));
        assert_eq!(fixture.reverse_resolve_domain(user_identity.clone()).await, Some(minted_asset_2));

        fixture.set_resolution(SUB_DOMAIN_1, Some(user_identity.clone())).await;
        fixture.set_primary(SUB_DOMAIN_1).await;

        assert_eq!(fixture.resolve_domain(SUB_DOMAIN_1).await, Some(user_identity.clone()));
        assert_eq!(fixture.resolve_domain(SUB_DOMAIN_2).await, Some(user_identity.clone()));
        assert_eq!(fixture.reverse_resolve_domain(user_identity.clone()).await, Some(minted_asset_1));
    }

    #[tokio::test]
    async fn test_funds_withdrawal() {
        let fixture = setup().await;
        for domain in ["abcde", "1238172", "aaaaa", "fuelet", "000000"].iter() {
            fixture.mint_domain(&domain, 1, COMMON_DEFAULT_FEE).await.unwrap();
        }
        let balance_before = fixture.deployer.get_asset_balance(&BASE_ASSET_ID).await.unwrap();
        fixture.withdraw_funds(&BASE_ASSET_ID).await;
        let balance_after = fixture.deployer.get_asset_balance(&BASE_ASSET_ID).await.unwrap();
        assert_eq!(balance_after - balance_before, COMMON_DEFAULT_FEE * 5 - 1);
    }

    #[tokio::test]
    async fn test_funds_withdrawal_other_assets() {
        let fixture = setup().await;
        fixture.set_fees(&usdc_asset_id(), 1000, 100, 10).await;
        for domain in ["abcde", "1238172", "aaaaa", "fuelet", "000000"].iter() {
            fixture._mint_domain(&domain, 1, 10, Some(usdc_asset_id())).await.unwrap();
        }
        let balance_before = fixture.deployer.get_asset_balance(&usdc_asset_id()).await.unwrap();
        fixture.withdraw_funds(&usdc_asset_id()).await;
        let balance_after = fixture.deployer.get_asset_balance(&usdc_asset_id()).await.unwrap();
        assert_eq!(balance_after - balance_before, 10 * 5);
    }

    #[tokio::test]
    async fn test_set_resolver() {
        let fixture = setup().await;
        let original_resolver: ContractId = fixture.resolver_contract.contract_id().clone().into();
        let new_resolver = ContractId::new(random());
        let additional_resolver = ContractId::new(random());

        let asset = fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
        let domain_name = fixture.get_domain_name(asset).await;

        let default_resolver = fixture.get_domain_resolver(&domain_name).await.unwrap();
        assert_eq!(default_resolver, original_resolver);

        fixture.set_domain_resolver(&domain_name, new_resolver).await;
        assert_eq!(fixture.get_domain_resolver(&domain_name).await.unwrap(), new_resolver);

        fixture.set_domain_resolver(&domain_name, original_resolver).await;
        assert_eq!(fixture.get_domain_resolver(&domain_name).await.unwrap(), original_resolver);

        fixture.set_domain_resolver(&domain_name, additional_resolver).await;
        assert_eq!(fixture.get_domain_resolver(&domain_name).await.unwrap(), additional_resolver);
    }

    #[tokio::test]
    async fn test_set_fees() {
        let fixture = setup().await;
        let updated_three_letter_fee = 1000;
        let updated_four_letter_fee = 100;
        let updated_common_fee = 10;
        let asset_id = &BASE_ASSET_ID;
        assert_eq!(fixture.get_domain_price("fue", 1, asset_id).await, THREE_LETTER_ANNUAL_DEFAULT_FEE);
        assert_eq!(fixture.get_domain_price("fuel", 1, asset_id).await, FOUR_LETTER_ANNUAL_DEFAULT_FEE);
        assert_eq!(fixture.get_domain_price("fuelet", 1, asset_id).await, COMMON_ANNUAL_DEFAULT_FEE);

        fixture.mint_domain("fue", 1, THREE_LETTER_ANNUAL_DEFAULT_FEE).await.unwrap();
        fixture.mint_domain("fuel", 1, FOUR_LETTER_ANNUAL_DEFAULT_FEE).await.unwrap();
        fixture.mint_domain("fuelet", 1, COMMON_ANNUAL_DEFAULT_FEE).await.unwrap();

        fixture.set_fees(asset_id, updated_three_letter_fee, updated_four_letter_fee, updated_common_fee).await;

        assert_eq!(fixture.get_domain_price("fue", 1, asset_id).await, updated_three_letter_fee);
        assert_eq!(fixture.get_domain_price("fuel", 1, asset_id).await, updated_four_letter_fee);
        assert_eq!(fixture.get_domain_price("fuelet", 1, asset_id).await, updated_common_fee);

        fixture.mint_domain("euf", 1, updated_three_letter_fee).await.unwrap();
        fixture.mint_domain("leuf", 1, updated_four_letter_fee).await.unwrap();
        fixture.mint_domain("teleuf", 1, updated_common_fee).await.unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn test_set_primary_if_no_resolution_set() {
        let fixture = setup().await;
        fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();

        fixture.set_primary(SUB_DOMAIN_1).await;
    }

    #[tokio::test]
    #[should_panic]
    async fn test_set_primary_if_wrong_resolution_set() {
        let fixture = setup().await;
        fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
        let deployer_identity = Identity::Address(fixture.deployer.address().into());

        fixture.set_resolution(SUB_DOMAIN_1, Some(deployer_identity)).await;

        fixture.set_primary(SUB_DOMAIN_1).await;
    }

    #[tokio::test]
    async fn test_set_primary_if_correct_resolution_set() {
        let fixture = setup().await;
        let user_identity = Identity::Address(fixture.user.address().into());
        let domain_asset = fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();

        fixture.set_resolution(SUB_DOMAIN_1, Some(user_identity.clone())).await;

        fixture.set_primary(SUB_DOMAIN_1).await;
        assert_eq!(fixture.reverse_resolve_domain(user_identity).await, Some(domain_asset));
    }

    #[tokio::test]
    #[should_panic]
    async fn test_set_primary_if_expired() {
        let fixture = setup().await;
        let user_identity = Identity::Address(fixture.user.address().into());
        fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
        fixture.set_resolution(SUB_DOMAIN_1, Some(user_identity.clone())).await;
        fixture.skip_n_days(380, true).await;
        fixture.set_primary(SUB_DOMAIN_1).await;
    }

    #[tokio::test]
    #[should_panic]
    async fn test_set_primary_if_resolution_is_not_set() {
        let fixture = setup().await;
        fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
        fixture.set_primary(SUB_DOMAIN_1).await;
    }

    #[tokio::test]
    #[should_panic]
    async fn test_set_primary_if_reset() {
        let fixture = setup().await;
        let user_identity = Identity::Address(fixture.user.address().into());
        let deployer_identity = Identity::Address(fixture.deployer.address().into());
        fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
        fixture.set_resolution(SUB_DOMAIN_1, Some(user_identity.clone())).await;
        fixture.set_primary(SUB_DOMAIN_1).await;
        fixture.set_resolution(SUB_DOMAIN_1, Some(deployer_identity.clone())).await;
        fixture.set_primary(SUB_DOMAIN_1).await;
    }

    #[tokio::test]
    #[should_panic]
    async fn test_set_resolution_if_expired() {
        let fixture = setup().await;
        let user_identity = Identity::Address(fixture.user.address().into());
        fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
        fixture.skip_n_days(380, true).await;
        fixture.set_resolution(SUB_DOMAIN_1, Some(user_identity.clone())).await;
    }

    #[tokio::test]
    async fn test_successful_reverse_resolution() {
        let fixture = setup().await;
        let user_identity = Identity::Address(fixture.user.address().into());
        let asset = fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
        fixture.set_resolution(SUB_DOMAIN_1, Some(user_identity.clone())).await;
        fixture.set_primary(SUB_DOMAIN_1).await;
        let reverse = fixture.reverse_resolve_domain(user_identity.clone()).await;
        assert_eq!(reverse, Some(asset));
    }

    #[tokio::test]
    async fn test_reverse_resolution_if_expired() {
        // should be available for now as long as the old address is unchanged
        let fixture = setup().await;
        let user_identity = Identity::Address(fixture.user.address().into());
        fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
        fixture.set_resolution(SUB_DOMAIN_1, Some(user_identity.clone())).await;
        fixture.set_primary(SUB_DOMAIN_1).await;
        fixture.skip_n_days(380, true).await;
        let reverse = fixture.reverse_resolve_domain(user_identity.clone()).await;
        assert_eq!(reverse, None);
    }

    #[tokio::test]
    async fn test_reverse_resolution_if_unset() {
        let fixture = setup().await;
        let user_identity = Identity::Address(fixture.user.address().into());
        let deployer_identity = Identity::Address(fixture.deployer.address().into());
        fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
        fixture.set_resolution(SUB_DOMAIN_1, Some(user_identity.clone())).await;
        fixture.set_primary(SUB_DOMAIN_1).await;
        fixture.set_resolution(SUB_DOMAIN_1, Some(deployer_identity.clone())).await;
        let reverse = fixture.reverse_resolve_domain(user_identity.clone()).await;
        assert_eq!(reverse, None);
    }

    #[tokio::test]
    #[should_panic]
    async fn test_set_grace_period_not_owner() {
        let fixture = setup().await;
        fixture.set_grace_period_as_user(MIN_GRACE_PERIOD_DURATION + 1).await;
    }

    #[tokio::test]
    #[should_panic]
    async fn test_set_grace_period_less_than_min() {
        let fixture = setup().await;
        fixture.set_grace_period(MIN_GRACE_PERIOD_DURATION - 1).await;
    }

    #[tokio::test]
    async fn test_set_grace_period_happy_path() {
        let fixture = setup().await;
        let gp = MIN_GRACE_PERIOD_DURATION + 1000;
        fixture.set_grace_period(gp).await;
        assert_eq!(gp, fixture.get_grace_period().await);
    }

    #[tokio::test]
    async fn test_renew_happy_path() {
        let fixture = setup().await;
        fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
        let expiration_before = fixture.get_domain_expiration(SUB_DOMAIN_1).await.unwrap();
        fixture.renew_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await;
        assert_eq!(fixture.get_domain_expiration(SUB_DOMAIN_1).await.unwrap(), expiration_before + ONE_YEAR_SECONDS);
    }

    #[tokio::test]
    #[should_panic]
    async fn test_renew_inactive_domain() {
        let fixture = setup().await;
        fixture.renew_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await;
    }

    #[tokio::test]
    #[should_panic]
    async fn test_mint_while_active() {
        let fixture = setup().await;
        fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
        fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
    }

    #[tokio::test]
    async fn test_mint_after_expiration() {
        let fixture = setup().await;
        let initial_asset = fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
        fixture.skip_n_days(400, true).await; // more than exp + grace
        let new_asset = fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
        assert_ne!(initial_asset, new_asset);
    }

    #[tokio::test]
    #[should_panic]
    async fn test_mint_after_expiration_before_grace() {
        let fixture = setup().await;
        fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
        fixture.skip_n_days(380, true).await; // more than exp but less than exp + grace
        fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn test_mint_before_expiration_before_grace() {
        let fixture = setup().await;
        fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
        fixture.skip_n_days(340, true).await; // less than exp
        fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
    }

    #[tokio::test]
    #[should_panic]
    async fn test_wrong_domain_renewal() {
        let fixture = setup().await;
        fixture.mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE).await.unwrap();
        fixture.renew_domain(SUB_DOMAIN_1, 1, COMMON_DEFAULT_FEE).await;
    }

    #[tokio::test]
    #[should_panic(expected = "WrongFeeAsset")]
    async fn test_wrong_asset_get_price() {
        let fixture = setup().await;
        fixture.get_domain_price(SUB_DOMAIN_PART_1, 1, &usdc_asset_id()).await;
    }

    #[tokio::test]
    #[should_panic(expected = "WrongFeeAsset")]
    async fn test_wrong_asset_payment() {
        let fixture = setup().await;
        fixture._mint_domain(SUB_DOMAIN_PART_1, 1, COMMON_DEFAULT_FEE, Some(usdc_asset_id())).await.unwrap();
    }

    #[tokio::test]
    async fn test_different_asset_payment() {
        let fixture = setup().await;
        fixture.set_fees(&usdc_asset_id(), 1000, 100, 10).await;
        let price = fixture.get_domain_price(SUB_DOMAIN_PART_1, 1, &usdc_asset_id()).await;
        let balance = fixture.user.get_asset_balance(&usdc_asset_id()).await.unwrap();
        assert_eq!(price, 10);
        fixture._mint_domain(SUB_DOMAIN_PART_1, 1, 10, Some(usdc_asset_id())).await.unwrap();
        let updated_balance = fixture.user.get_asset_balance(&usdc_asset_id()).await.unwrap();
        assert_eq!(updated_balance, balance - 10);
    }

    #[tokio::test]
    #[should_panic(expected = "WrongFeeAmount")]
    async fn test_wrong_fee_amount() {
        let fixture = setup().await;
        fixture.set_fees(&usdc_asset_id(), 1000, 100, 10).await;
        let price = fixture.get_domain_price(SUB_DOMAIN_PART_1, 1, &usdc_asset_id()).await;
        assert_eq!(price, 10);
        fixture._mint_domain(SUB_DOMAIN_PART_1, 1, 11, Some(usdc_asset_id())).await.unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "WrongFeeAsset")]
    async fn test_remove_fee_asset() {
        let fixture = setup().await;
        fixture.remove_fee_asset(&BASE_ASSET_ID).await;
        assert!(fixture.mint_domain(SUB_DOMAIN_PART_2, 1, COMMON_DEFAULT_FEE).await.is_err());
        fixture.get_domain_price(SUB_DOMAIN_PART_2, 1, &BASE_ASSET_ID).await;
    }
}
