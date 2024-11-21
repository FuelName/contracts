use deploy::deployer::{ContractType, DeployResult};
use deploy::fixture::Fixture;
use fuels::prelude::ContractId;
use maplit::hashmap;
use std::collections::HashMap;
use std::str::FromStr;
use deploy::shared::{config, get_wallets};

#[tokio::main]
async fn main() {
    let contracts: HashMap<ContractType, DeployResult> = hashmap! {
        ContractType::Resolver => DeployResult {
            target_id: id("0x73224c62ca8acb46b732067bfd4a7eb2a98b095388552aee4aa846bff2124ef5"),
            proxy_id: id("0xb2fd140aa227685c65104c583b75dca9380d2936ffe2285d26581548420745d5")
        },
        ContractType::Registrar => DeployResult {
            target_id: id("0xf97d1ffc76d87c90f4a9593f88c1cdeb306367761ee595e7b84e36fedeac3d40"),
            proxy_id: id("0xc8e8b27dbaab3a6679e39d241f21bbdcc68f9951fbf30693fb9efc9527454fa1")
        },
        ContractType::Registry => DeployResult {
            target_id: id("0x4fc93a4820df7c597203317a5102bc0a1d0bd0e2e95268b829326d581589c5d3"),
            proxy_id: id("0x22d75396a36c00148a704094804bba1cd09816d788f81c260c868d2f4565aa30")
        },
    };
    let config = config();
    let (deployer, user) = get_wallets(&config).await;
    let fixture = Fixture::connect(deployer, user, contracts);

    // mint_reserved_domains(fixture).await;
    call_on_chain_function(fixture).await;
}

fn id(str: &str) -> ContractId {
    ContractId::from_str(str).unwrap()
}

async fn call_on_chain_function(fixture: Fixture) {
    // let domain_name = fixture.get_domain_name(AssetId::from_str("0xb0e42e49bcc1bc732be8b55fcf015e2e4093a4e36f83c827b4451b15c8cd50f9").unwrap()).await;
    // println!("{:?}", domain_name);

    // fixture.withdraw_funds(&AssetId::BASE).await;

    let total_assets = fixture.get_total_assets().await;
    println!("Total assets: {}", total_assets);

    // let asset_id = fixture.get_domain_asset_id("out.fuel").await;
    // println!("Asset ID: {:?}", asset_id);
    // let uri = fixture.get_domain_uri(asset_id).await;
    // println!("URI: {:?}", uri);

    // fixture.transfer(&fixture.user, "dino.fuel", &Bech32Address::from_str("fuel1xvwtd4tz3509kugtxx783kd2rrywyqcwper54sku8v7x5hgw7axq6xduf3").unwrap()).await;
}

async fn mint_reserved_domains(fixture: Fixture) {
    let reserved_domains = vec!["wallet", "fuelnameservice", "fns", "fueldomains", "domains", "thunder", "spark", "swaylend", "bsafe", "sway", "fuel", "fuelnetwork"];
    for domain in reserved_domains {
        let asset = fixture._mint_domain(domain, 3, 0, None).await.unwrap();
        println!("{}: {}", domain, asset);
    }
}
