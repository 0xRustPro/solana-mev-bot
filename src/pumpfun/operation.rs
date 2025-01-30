use anyhow::{anyhow, Result};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature},
    signer::Signer,
    transaction::Transaction,
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use std::{env, sync::Arc};

use crate::{
    constants::accounts::TOKEN_PROGRAM,
    new_client,
    pumpfun::{
        instructions::{create_buy_instruction, create_sell_instruction},
        math::amount_with_slippage,
        utils::{get_bonding_curve_account, get_global_account},
    },
};

pub async fn buy(
    client: Arc<RpcClient>,
    payer: &Keypair,
    mint: &Pubkey,
    amount_sol: u64,
    slippage: u64,
    is_simulate: bool,
) -> Result<Vec<Signature>> {
    let mut instructions = vec![];
    // 计算数量
    let bonding_curve_account = get_bonding_curve_account(client.clone(), mint).await?;
    let buy_amount = bonding_curve_account.get_buy_price(amount_sol).unwrap();

    // 滑点
    let buy_amount_with_slippage = amount_with_slippage(buy_amount, slippage * 100, true)?;

    // 获取关联账户
    let mint_ata = get_associated_token_address(&payer.pubkey(), &mint);
    println!("mint_ata {:?}", mint_ata);

    // 获取不到关联账户，需要创建
    if client.get_account(&mint_ata).await.is_err() {
        instructions.push(create_associated_token_account(
            &payer.pubkey(),
            &payer.pubkey(),
            &mint,
            &TOKEN_PROGRAM,
        ));
    }

    // buy指令
    instructions.push(create_buy_instruction(
        payer,
        mint,
        buy_amount,
        buy_amount_with_slippage,
    ));
    let recent_blockhash = client.get_latest_blockhash().await.unwrap();

    // 创建交易
    let txn = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &[payer],
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
    } else {
        let res = client.send_transaction(&txn).await?;
        Ok(vec![res])
    }
}

pub async fn sell(
    client: Arc<RpcClient>,
    payer: &Keypair,
    mint: &Pubkey,
    amount_token: u64,
    slippage: u64,
    is_simulate: bool,
) -> Result<Vec<Signature>> {
    // 获取当前账户余额
    let payer_pub_key = &payer.pubkey();
    let ata = get_associated_token_address(payer_pub_key, mint);
    let token_balance = client.get_token_account_balance(&ata).await?;
    let token_balance_u64 = token_balance.ui_amount.unwrap() as u64;

    assert!(token_balance_u64 >= amount_token);

    // bonding curve
    let bonding_curve = get_bonding_curve_account(client.clone(), mint).await?;
    // 全局账户
    let global_account = get_global_account(client.clone()).await?;

    let sol_output = bonding_curve
        .get_sell_price(amount_token, global_account.fee_basis_points)
        .unwrap();
    let min_sol_output = amount_with_slippage(sol_output, slippage * 100, false).unwrap();

    // 创建sell指令
    let mut instructions = vec![];
    instructions.push(create_sell_instruction(
        payer,
        mint,
        sol_output,
        min_sol_output,
    ));
    let recent_blockhash = client.get_latest_blockhash().await.unwrap();

    // 创建交易
    let txn = Transaction::new_signed_with_payer(
        &instructions,
        Some(&payer.pubkey()),
        &[payer],
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
    } else {
        let res = client.send_transaction(&txn).await?;
        Ok(vec![res])
    }
}

#[tokio::test]
async fn test_buy() {
    dotenv::dotenv().ok();
    let keypair = Keypair::from_base58_string(&env::var("PK").unwrap());
    let mint = Pubkey::from_str_const("8vbjWGXKhrKfVMCXpLrUGyUUHKNfmvRiuT2Dn2h1pump");

    let client = new_client();
    buy(client, &keypair, &mint, 1, 2, true).await.unwrap();
}

#[tokio::test]
async fn test_sell() {
    dotenv::dotenv().ok();
    let keypair = Keypair::from_base58_string(&env::var("PK").unwrap());
    let mint = Pubkey::from_str_const("8vbjWGXKhrKfVMCXpLrUGyUUHKNfmvRiuT2Dn2h1pump");

    let client = new_client();
    sell(client, &keypair, &mint, 1, 2, true).await.unwrap();
}
