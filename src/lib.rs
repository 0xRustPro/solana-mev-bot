mod constants;
mod engine;
mod monitor;
mod pumpfun;
mod raydium;
mod strategy;

pub use monitor::token_create::listen_pumpfun_create;
pub use monitor::token_migration::listen_rayidum_migration;

pub fn new_client() -> std::sync::Arc<solana_client::nonblocking::rpc_client::RpcClient> {
    dotenv::dotenv().ok();
    std::sync::Arc::new(solana_client::nonblocking::rpc_client::RpcClient::new(
        std::env::var("RPC_URL").unwrap(),
    ))
}

pub async fn new_ws_client(
) -> anyhow::Result<std::sync::Arc<solana_client::nonblocking::pubsub_client::PubsubClient>> {
    dotenv::dotenv().ok();
    Ok(std::sync::Arc::new(
        solana_client::nonblocking::pubsub_client::PubsubClient::new(
            std::env::var("WS_RPC_URL").unwrap().as_str(),
        )
        .await?,
    ))
}
