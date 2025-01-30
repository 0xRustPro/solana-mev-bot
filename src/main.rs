use raydium_swap::{listen_pumpfun_create, listen_rayidum_migration, new_ws_client};

#[tokio::main]
async fn main() {
    let ws_client = new_ws_client().await.unwrap();
    let set = listen_pumpfun_create(ws_client, 1000).await.unwrap();
    set.join_all().await;
}
