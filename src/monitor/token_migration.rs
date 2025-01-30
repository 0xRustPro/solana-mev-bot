use std::sync::Arc;

use crate::pumpfun::utils::get_bonding_curve_account;
use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use solana_client::{
    nonblocking::{pubsub_client::PubsubClient, rpc_client::RpcClient},
    rpc_config::{RpcBlockSubscribeConfig, RpcBlockSubscribeFilter},
};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use solana_transaction_status_client_types::{EncodedTransactionWithStatusMeta, UiConfirmedBlock};
use teloxide::{
    payloads::SendMessageSetters,
    prelude::Requester,
    types::{ChatId, ParseMode},
    Bot,
};
use tokio::{sync::broadcast, task::JoinSet};

const CHATID: i64 = 1233301525;
const PUMPFUNMIGRATOR: &str = "39azUYFWPz3VHgKCf3VChUwbpURdCHRxjWVowf5jUJjg";

/// 检查mint代币的状态
pub async fn check_token_status(client: Arc<RpcClient>, mint: &str) -> Result<bool> {
    let mint = Pubkey::from_str_const(mint);
    let bonding_curve = get_bonding_curve_account(client, &mint).await?;
    Ok(bonding_curve.complete)
}

pub fn process_initialize2_transaction(tx: &EncodedTransactionWithStatusMeta) -> Option<String> {
    let decode_tx = tx.transaction.decode().unwrap();
    let signature = decode_tx.signatures[0];
    let account_keys = decode_tx.message.static_account_keys();
    if account_keys.len() > 19 {
        let coin_token = account_keys[18];
        let pc_token = account_keys[19];
        let liquidity_address = account_keys[2];

        println!("signature {:?}", signature.to_string());
        println!("coin_token address {:?}", coin_token);
        println!("pc_token address {:?}", pc_token);
        println!("Liquidity address {:?}", liquidity_address);
        println!("==============================================================================================");
        return Some(format!(
            "**🚀 Token Migration 🚀**\n\
            ```\n\
            signature:           {}\n\
            coin_token address:  {:?}\n\
            pc_token address:    {:?}\n\
            Liquidity address:   {:?}\n\
            ```",
            signature.to_string(),
            coin_token,
            pc_token,
            liquidity_address
        ));
    } else {
        None
    }
}

pub fn process_block(block: UiConfirmedBlock) -> Vec<String> {
    let mut result = vec![];
    for tx in block.transactions.unwrap() {
        let logs = tx.meta.as_ref().unwrap().log_messages.clone().unwrap();
        for log in logs {
            if log.contains("Program log: initialize2: InitializeInstruction2") {
                println!("Found initialize2 instruction!");
                let res = process_initialize2_transaction(&tx);
                if res.is_some() {
                    result.push(res.unwrap());
                }
            }
        }
    }
    result
}

pub async fn listen_rayidum_migration(
    ws_client: Arc<PubsubClient>,
    channel_size: usize,
) -> Result<JoinSet<()>> {
    let mut set: JoinSet<()> = JoinSet::new();
    let (block_sender, _) = broadcast::channel(channel_size);
    let bot = Arc::new(Bot::from_env());

    // 处理log的线程
    let mut block_receiver = block_sender.subscribe();
    set.spawn(async move {
        while let Ok(block) = block_receiver.recv().await {
            let result = process_block(block);
            for res in result {
                // 发送到tgbot
                match bot
                    .send_message(ChatId(CHATID), res)
                    .parse_mode(ParseMode::MarkdownV2)
                    .await
                {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("send to bot error {:?}", e);
                    }
                }
            }
        }
    });

    // 发出block的线程
    set.spawn(async move {
        let (mut stream, _) = ws_client
            .block_subscribe(
                // 只关注migrator
                // RpcBlockSubscribeFilter::MentionsAccountOrProgram(PUMPFUNMIGRATOR.to_string()),
                RpcBlockSubscribeFilter::All,
                // 区块信息配置
                Some(RpcBlockSubscribeConfig {
                    commitment: Some(CommitmentConfig::confirmed()),
                    encoding: Some(
                        solana_transaction_status_client_types::UiTransactionEncoding::Binary,
                    ),
                    transaction_details: Some(
                        solana_transaction_status_client_types::TransactionDetails::Full,
                    ),
                    show_rewards: Some(false),
                    max_supported_transaction_version: Some(0),
                }),
            )
            .await
            .map_err(|e| anyhow!("failed to get stream {:?}", e))
            .unwrap();

        // 发送block
        while let Some(new_block) = stream.next().await {
            if let Some(block) = new_block.value.block {
                match block_sender.send(block) {
                    Ok(_) => {}
                    Err(e) => {
                        eprintln!("send block error")
                    }
                }
            }
        }
    });

    // 返回set到主线程
    Ok(set)
}
