use std::{env, sync::Arc};

use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{
    instruction::Instruction, program_pack::Pack, pubkey::Pubkey, signature::Keypair,
    signer::Signer, system_instruction,
};
use spl_associated_token_account::{
    get_associated_token_address, instruction::create_associated_token_account,
};
use spl_token::{amount_to_ui_amount, state::Account, ui_amount_to_amount};

use crate::{
    new_client,
    raydium::{getter, math::calculate_swap_info, swap_instructions, tx::new_signed_and_send},
};

use super::{
    getter::get_pool_state,
    structure::{AmmSwapInfoResult, SwapDirection},
};
pub const AMM_PROGRAM: &str = "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8";

pub async fn get_swap_tx(
    client: Arc<RpcClient>,
    token_in: &str,
    token_out: &str,
    amount_in: f64,
    pool_id: &str,
    slippage: u64,
    keypair: Arc<Keypair>,
) -> Result<()> {
    // 滑点
    let slippage_bps = slippage * 100;
    // 用户pubkey
    let owner = keypair.pubkey();

    let token_in = Pubkey::from_str_const(token_in);
    let token_out = Pubkey::from_str_const(token_out);

    // 原生程序
    let program_id = spl_token::ID;
    let native_mint = spl_token::native_mint::ID;

    // 获取池子状态
    let (pool_id, pool_state) = get_pool_state(client.clone(), pool_id).await?;

    let coin_mint = pool_state.coin_vault_mint;
    let pc_mint = pool_state.pc_vault_mint;

    let coin_vault = pool_state.coin_vault;
    let pc_vault = pool_state.pc_vault;

    // swap方向
    let (user_input_token, swap_direction) = if token_in.eq(&coin_mint) {
        // 使用sol购买代币
        assert_eq!(token_out, pc_mint);
        (coin_vault, SwapDirection::Buy)
    } else {
        // 使用代币购买sol
        assert_eq!(token_out, coin_mint);
        (pc_vault, SwapDirection::Sell)
    };

    // swap base in
    let swap_base_in = token_in == native_mint;

    // 获取ata地址
    let in_ata = get_associated_token_address(&owner, &token_in);
    let out_ata = get_associated_token_address(&owner, &token_out);

    let mut create_instruction = None;

    // 计算出输入数量的准确数值
    let (amount_specified, _) = match swap_direction {
        SwapDirection::Buy => {
            // 获取输出代币的ATA地址的账户信息
            match getter::get_account_info(client.clone(), keypair.clone(), &token_out, &out_ata)
                .await
            {
                Ok(_) => {}
                Err(_) => {
                    // 获取账户失败，创建ata账户
                    create_instruction = Some(create_associated_token_account(
                        &owner,
                        &owner,
                        &token_out,
                        &program_id,
                    ));
                }
            };
            (
                ui_amount_to_amount(amount_in, spl_token::native_mint::DECIMALS),
                (amount_in, spl_token::native_mint::DECIMALS),
            )
        }
        SwapDirection::Sell => {
            // 卖出
            let in_mint = getter::get_mint_info(client.clone(), keypair.clone(), &token_in).await?;
            // println!("in_mint {:?}", in_mint);
            let amount = ui_amount_to_amount(amount_in, in_mint.decimals);
            (
                amount,
                (
                    amount_to_ui_amount(amount, in_mint.decimals),
                    in_mint.decimals,
                ),
            )
        }
    };

    // amm program
    let amm_program = Pubkey::from_str_const(AMM_PROGRAM);

    // 模拟swap后的结果
    let swap_info_result = calculate_swap_info(
        client.clone(),
        &pool_state,
        amm_program,
        pool_id,
        user_input_token,
        amount_specified,
        slippage_bps,
        swap_base_in,
    )
    .await?;
    let other_amount_threshold = swap_info_result.other_amount_threshold;
    // println!("other number {:?}", swap_info_result.other_amount_threshold);

    let mut instructions = vec![];
    // 可能需要wsol账户
    let mut wsol_account = None;
    // 如果输入输出是sol，需要创建wsol账户
    if token_in == native_mint || token_out == native_mint {
        // 账户计算
        let seed = &format!("{}", Keypair::new().pubkey())[..32];
        let wsol_pubkey = Pubkey::create_with_seed(&owner, seed, &spl_token::id())?;
        wsol_account = Some(wsol_pubkey);

        // LAMPORTS_PER_SOL / 100 // 0.01 SOL as rent

        // 获取租金
        let rent = client
            .clone()
            .get_minimum_balance_for_rent_exemption(Account::LEN)
            .await?;
        // 计算要转入wsol账户的sol数量
        let total_amount = if token_in == native_mint {
            rent + amount_specified
        } else {
            rent
        };
        // println!("total_amount {:?}", total_amount);
        // 创建wsol账户
        // 此处为临时的
        instructions.push(system_instruction::create_account_with_seed(
            &owner,
            &wsol_pubkey,
            &owner,
            seed,
            total_amount,
            Account::LEN as u64, // 165, // Token account size
            &spl_token::id(),
        ));

        // initialize account
        // 初始化账户
        instructions.push(spl_token::instruction::initialize_account(
            &spl_token::id(),
            &wsol_pubkey,
            &native_mint,
            &owner,
        )?);
    }

    // 创建指令
    if let Some(create_instruction) = create_instruction {
        instructions.push(create_instruction);
    }

    if amount_specified > 0 {
        let mut close_wsol_account_instruction = None;
        // replace native mint with tmp wsol account
        let mut final_in_ata = in_ata;
        let mut final_out_ata = out_ata;

        // 如果是和sol相关，之后需要关闭wsol账户
        if let Some(wsol_account) = wsol_account {
            match swap_direction {
                SwapDirection::Buy => {
                    // buy，token_in的ata是wsol的
                    final_in_ata = wsol_account;
                }
                SwapDirection::Sell => {
                    // sell，token_out的ata是wsol的
                    final_out_ata = wsol_account;
                }
            }
            close_wsol_account_instruction = Some(spl_token::instruction::close_account(
                &program_id,
                &wsol_account,
                &owner,
                &owner,
                &vec![&owner],
            )?);
        }

        // swap指令
        let build_swap_instruction = amm_swap(
            &amm_program,
            swap_info_result,
            &owner,
            &final_in_ata,
            &final_out_ata,
            amount_specified,
            other_amount_threshold,
            swap_base_in,
        )?;
        println!(
            "amount_specified: {}, other_amount_threshold: {}, wsol_account: {:?}",
            amount_specified, other_amount_threshold, wsol_account
        );
        instructions.push(build_swap_instruction);
        // close wsol account
        if let Some(close_wsol_account_instruction) = close_wsol_account_instruction {
            instructions.push(close_wsol_account_instruction);
        }
    }
    new_signed_and_send(client.clone(), keypair.clone(), instructions, true).await?;
    Ok(())
}

fn amm_swap(
    amm_program: &Pubkey,
    result: AmmSwapInfoResult,
    user_owner: &Pubkey,
    user_source: &Pubkey,
    user_destination: &Pubkey,
    amount_specified: u64,
    other_amount_threshold: u64,
    swap_base_in: bool,
) -> Result<Instruction> {
    let swap_instruction = if swap_base_in {
        swap_instructions::swap_base_in(
            &amm_program,
            &result.pool_id,
            &result.amm_authority,
            &result.amm_open_orders,
            &result.amm_coin_vault,
            &result.amm_pc_vault,
            &result.market_program,
            &result.market,
            &result.market_bids,
            &result.market_asks,
            &result.market_event_queue,
            &result.market_coin_vault,
            &result.market_pc_vault,
            &result.market_vault_signer,
            user_source,
            user_destination,
            user_owner,
            amount_specified,
            other_amount_threshold,
        )?
    } else {
        swap_instructions::swap_base_out(
            &amm_program,
            &result.pool_id,
            &result.amm_authority,
            &result.amm_open_orders,
            &result.amm_coin_vault,
            &result.amm_pc_vault,
            &result.market_program,
            &result.market,
            &result.market_bids,
            &result.market_asks,
            &result.market_event_queue,
            &result.market_coin_vault,
            &result.market_pc_vault,
            &result.market_vault_signer,
            user_source,
            user_destination,
            user_owner,
            other_amount_threshold,
            amount_specified,
        )?
    };

    Ok(swap_instruction)
}

#[tokio::test]
async fn test_get_swap_tx_in_raydium() -> Result<()> {
    // 模拟 RPC 客户端
    let client = new_client();

    // 模拟池子 ID
    let pool_id = "iJuiniVZc7rHYKcvEy9Dz5arHjjmrbfYLdY4etGfQXr"; // 替换为实际的池子 ID

    // 模拟输入金额
    let amount_in = 0.2;

    // 模拟滑点
    // 此时滑点0.1%
    let slippage = 1;

    // 模拟用户密钥对
    let keypair = Arc::new(Keypair::from_base58_string(&env::var("PK").unwrap()));

    // 调用函数
    let result = get_swap_tx(
        client,
        "So11111111111111111111111111111111111111112",
        "F9TgEJLLRUKDRF16HgjUCdJfJ5BK6ucyiW8uJxVPpump",
        amount_in,
        pool_id,
        slippage,
        keypair,
    )
    .await
    .unwrap();

    Ok(())
}
