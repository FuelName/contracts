use crate::deployer::DeployTarget;
mod deployer;
mod fixture;
mod shared;

#[tokio::main]
async fn main() {
    deployer::deploy(DeployTarget::OnChain).await;
}
