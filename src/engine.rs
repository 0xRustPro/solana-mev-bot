use anyhow::{anyhow, Result};
use solana_client::nonblocking::{pubsub_client::PubsubClient, rpc_client::RpcClient};
use std::{sync::Arc, time::Duration};
use teloxide::{prelude::Requester, types::ChatId, Bot};
use tokio::{sync::broadcast, task::JoinSet};
use tracing::{error, info};
use twitter_v2::TwitterApi;

use crate::{
    monitor::twitter::twitter_monitor::{auth_for_twitter, get_post_content, process_tweet},
    strategy::Strategy,
};

pub struct Engine {
    // tg bot
    tg_bot: Arc<Bot>,
    // tx client
    http_client: Arc<RpcClient>,
    // listen client
    ws_client: Arc<PubsubClient>,
    // twitter poll interval
    poll_interval: u64,
    // strategy
    strategy: Strategy,
    chat_id: ChatId,
}

impl Engine {
    // run
    // twitter account,user_id
    pub async fn run(self, x_accounts: Vec<u64>, channel_size: usize) -> Result<JoinSet<()>> {
        let mut set = JoinSet::new();

        // send tx to process
        let (tx_sender, _) = broadcast::channel(channel_size);

        // 2. send tx
        let mut tx_receiver = tx_sender.subscribe();
        set.spawn(async move {
            while let Ok(tx) = tx_receiver.recv().await {
                // send tx to node
                match self.http_client.send_transaction(&tx).await {
                    Ok(sig) => {
                        info!("a tx send success! {:?}", sig);
                        // send to tgbot
                        let _ = self
                            .tg_bot
                            .send_message(self.chat_id, format!("new tx send {:?}", sig))
                            .await;
                    }
                    Err(e) => {
                        error!("failed to send tx {:?}", e);
                    }
                }
            }
        });

        // 1. fetch info from twitter
        set.spawn(async move {
            loop {
                let api = TwitterApi::new(auth_for_twitter());
                for user in &x_accounts {
                    match get_post_content(&api, user).await {
                        Ok(tweet_list) => {
                            // analyze twitter
                            for tweet in tweet_list {
                                // get op by twitter and strategy
                                if let Some(op) = process_tweet(tweet, &self.strategy).await {
                                    match tx_sender.send(op) {
                                        Ok(_) => {
                                            info!("transaction prepare to send to node");
                                        }
                                        Err(e) => {
                                            error!("send transaction error {:?}", e);
                                        }
                                    }
                                }
                            }
                        }
                        Err(_) => {}
                    }
                }

                // wait
                tokio::time::sleep(Duration::from_secs(self.poll_interval)).await;
            }
        });
        Ok(set)
    }
}
