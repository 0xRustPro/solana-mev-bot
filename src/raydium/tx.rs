use std::{env, sync::Arc, time::Instant};

use anyhow::{anyhow, Result};
use solana_client::{nonblocking::rpc_client::RpcClient, rpc_config::RpcSendTransactionConfig};
use solana_sdk::{
    commitment_config::CommitmentConfig,
    instruction::Instruction,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use std::str::FromStr;
use tracing::info;

fn get_unit_price() -> u64 {
    env::var("UNIT_PRICE")
        .ok()
        .and_then(|v| u64::from_str(&v).ok())
        .unwrap_or(20000)
}

fn get_unit_limit() -> u32 {
    env::var("UNIT_LIMIT")
        .ok()
        .and_then(|v| u32::from_str(&v).ok())
        .unwrap_or(200_000)
}

pub async fn new_signed_and_send(
    client: Arc<RpcClient>,
    keypair: Arc<Keypair>,
    mut instructions: Vec<Instruction>,
    is_simulate: bool,
) -> Result<Vec<String>> {
    let unit_limit = get_unit_limit();
    let unit_price = get_unit_price();
    // If not using Jito, manually set the compute unit price and limit
    let modify_compute_units =
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_limit(unit_limit);
    let add_priority_fee =
        solana_sdk::compute_budget::ComputeBudgetInstruction::set_compute_unit_price(unit_price);
    instructions.insert(0, modify_compute_units);
    instructions.insert(1, add_priority_fee);
    // send init tx
    let recent_blockhash = client.get_latest_blockhash().await?;
    let txn = Transaction::new_signed_with_payer(
        &instructions,
        Some(&keypair.pubkey()),
        &vec![&*keypair],
        recent_blockhash,
    );

    if is_simulate {
        let simulate_result = client.simulate_transaction(&txn).await?;
        if let Some(logs) = simulate_result.value.logs {
            for log in logs {
                println!("{}", log);
            }
        }
        return match simulate_result.value.err {
            Some(err) => Err(anyhow!("{}", err)),
            None => Ok(vec![]),
        };
    }

    let start_time = Instant::now();
    let mut txs = vec![];

    let sig = send_txn(&client, &txn, true).await?;
    info!("signature: {:?}", sig);
    txs.push(sig.to_string());

    info!("tx elapsed: {:?}", start_time.elapsed());

    Ok(txs)
}

pub async fn send_txn(
    client: &RpcClient,
    txn: &Transaction,
    skip_preflight: bool,
) -> Result<Signature> {
    Ok(client
        .send_and_confirm_transaction_with_spinner_and_config(
            txn,
            CommitmentConfig::confirmed(),
            RpcSendTransactionConfig {
                skip_preflight,
                ..RpcSendTransactionConfig::default()
            },
        )
        .await?)
}
