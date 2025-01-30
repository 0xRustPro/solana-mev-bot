use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use solana_client::{
    nonblocking::pubsub_client::PubsubClient,
    rpc_config::{RpcBlockSubscribeConfig, RpcBlockSubscribeFilter},
};
use solana_sdk::{commitment_config::CommitmentConfig, pubkey::Pubkey};
use solana_transaction_status_client_types::UiConfirmedBlock;
use std::str;
use std::sync::Arc;
use teloxide::{
    payloads::SendMessageSetters,
    prelude::Requester,
    types::{ChatId, ParseMode},
    Bot,
};
use tokio::{sync::broadcast, task::JoinSet};
const CHATID: i64 = 1233301525;

const PUMPFUNPROGRAM: Pubkey =
    Pubkey::from_str_const("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");

const CREATEDISCRIMINATOR: u64 = u64::from_le_bytes([24, 30, 200, 40, 5, 28, 7, 119]);
const IX_DEF: [(&str, &str); 3] = [("name", "string"), ("symbol", "string"), ("uri", "string")];

fn decode_create_instruction(ix_data: &[u8], accounts: Vec<String>) -> Result<String> {
    let mut args = Vec::new(); // 使用 Vec 保持顺序
    let mut offset = 8; // Skip 8-byte discriminator

    for (name, arg_type) in IX_DEF {
        match arg_type {
            "string" => {
                let length = u32::from_le_bytes(ix_data[offset..offset + 4].try_into()?) as usize;
                offset += 4;
                let value = str::from_utf8(&ix_data[offset..offset + length])?.to_string();
                offset += length;
                args.push((name.to_string(), value)); // 按顺序插入
            }
            "publicKey" => {
                let value = bs64::encode(&ix_data[offset..offset + 32]);
                offset += 32;
                args.push((name.to_string(), value)); // 按顺序插入
            }
            _ => return Err(anyhow!("Unsupported type: {:?}", arg_type).into()),
        }
    }

    // Add accounts in the correct order
    args.push(("mint".to_string(), accounts[0].clone()));
    args.push(("bondingCurve".to_string(), accounts[2].clone()));
    args.push(("associatedBondingCurve".to_string(), accounts[3].clone()));
    args.push(("user".to_string(), accounts[7].clone()));

    // Format as a beautiful Markdown string
    let mut markdown = String::new();
    markdown.push_str("**🚀 Token Create 🚀**\n");
    markdown.push_str("```\n");
    for (key, value) in args {
        markdown.push_str(&format!("{:25}: {}\n", key, value)); // 对齐输出
    }
    markdown.push_str("```");

    Ok(markdown)
}

pub fn process_block(block: UiConfirmedBlock) -> Vec<String> {
    let mut result = vec![];
    for tx in block.transactions.unwrap() {
        let tx = tx.transaction.decode().unwrap();
        let instructions = tx.message.instructions();
        let account_keys = tx.message.static_account_keys();
        for instruction in instructions {
            if account_keys[instruction.program_id_index as usize].eq(&PUMPFUNPROGRAM) {
                let slice = &instruction.data[..8];
                // 创建一个固定长度的数组
                let mut array = [0u8; 8];
                // 将切片内容复制到数组中
                array.copy_from_slice(slice);
                let discriminator = u64::from_le_bytes(array);
                if discriminator == CREATEDISCRIMINATOR {
                    // 相关账户收集
                    let accounts = instruction
                        .accounts
                        .iter()
                        .map(|idx| account_keys[*idx as usize].to_string())
                        .collect::<Vec<_>>();
                    // 处理指令

                    decode_create_instruction(&instruction.data, accounts)
                        .map(|v| result.push(v))
                        .unwrap();
                }
            }
        }
    }
    result
}

pub async fn listen_pumpfun_create(
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
                        eprintln!("send block error {:?}", e);
                    }
                }
            }
        }
    });

    // 返回set到主线程
    Ok(set)
}
