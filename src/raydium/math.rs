use std::sync::Arc;

use super::structure::{AmmInfo, AmmKeys, AmmSwapInfoResult};

use crate::raydium::swap_instructions::AmmInstruction::{SwapBaseIn, SwapBaseOut};
use crate::raydium::{
    getter::get_multiple_accounts,
    structure::{AmmStatus, SwapDirection},
};
use anyhow::{anyhow, Result};
use arrayref::array_ref;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey};
use spl_token::state::Account;

pub const AUTHORITY_AMM: &'static [u8] = b"amm authority";

pub async fn calculate_swap_info(
    rpc_client: Arc<RpcClient>,
    amm_state: &AmmInfo,
    amm_program: Pubkey,
    pool_id: Pubkey,
    user_input_token: Pubkey,
    amount_specified: u64,
    slippage_bps: u64,
    base_in: bool,
) -> Result<AmmSwapInfoResult> {
    // load amm keys
    let amm_keys = load_amm_keys(amm_state, &amm_program, &pool_id)?;
    let load_pubkeys = vec![
        pool_id,
        amm_keys.amm_pc_vault,
        amm_keys.amm_coin_vault,
        user_input_token,
    ];

    let rsps = get_multiple_accounts(rpc_client.clone(), &load_pubkeys).await?;
    let accounts = array_ref![rsps, 0, 4];
    let [amm_account, amm_pc_vault_account, amm_coin_vault_account, user_input_token_account] =
        accounts;
    let amm_pc_vault = Account::unpack(&amm_pc_vault_account.as_ref().unwrap().data).unwrap();
    let amm_coin_vault = Account::unpack(&amm_coin_vault_account.as_ref().unwrap().data).unwrap();
    let user_input_token_info =
        Account::unpack(&user_input_token_account.as_ref().unwrap().data).unwrap();
    assert_eq!(
        AmmStatus::from_u64(amm_state.status).orderbook_permission(),
        false
    );

    let (amm_pool_pc_vault_amount, amm_pool_coin_vault_amount) =
        calc_total_without_take_pnl_no_orderbook(
            amm_pc_vault.amount,
            amm_coin_vault.amount,
            &amm_state,
        )
        .unwrap();

    let (swap_direction, input_mint, output_mint) =
        if user_input_token_info.mint == amm_keys.amm_coin_mint {
            (
                SwapDirection::Buy,
                amm_keys.amm_coin_mint,
                amm_keys.amm_pc_mint,
            )
        } else if user_input_token_info.mint == amm_keys.amm_pc_mint {
            (
                SwapDirection::Sell,
                amm_keys.amm_pc_mint,
                amm_keys.amm_coin_mint,
            )
        } else {
            panic!("input tokens not match pool vaults");
        };

    let other_amount_threshold = swap_with_slippage(
        amm_pool_pc_vault_amount,
        amm_pool_coin_vault_amount,
        amm_state.fees.swap_fee_numerator,
        amm_state.fees.swap_fee_denominator,
        swap_direction,
        amount_specified,
        base_in,
        slippage_bps,
    )?;

    Ok(AmmSwapInfoResult {
        pool_id,
        amm_authority: amm_keys.amm_authority,
        amm_open_orders: amm_keys.amm_open_order,
        amm_coin_vault: amm_keys.amm_coin_vault,
        amm_pc_vault: amm_keys.amm_pc_vault,
        input_mint,
        output_mint,
        market_program: amm_keys.amm_authority, // padding readonly account
        market: amm_keys.amm_open_order,        // padding readwrite account
        market_coin_vault: amm_keys.amm_open_order, // padding readwrite account
        market_pc_vault: amm_keys.amm_open_order, // padding readwrite account
        market_vault_signer: amm_keys.amm_authority, // padding readonly account
        market_event_queue: amm_keys.amm_open_order, // padding readwrite account
        market_bids: amm_keys.amm_open_order,   // padding readwrite account
        market_asks: amm_keys.amm_open_order,   // padding readwrite account
        amount_specified,
        other_amount_threshold,
    })
}

pub fn load_amm_keys(amm: &AmmInfo, amm_program: &Pubkey, amm_pool: &Pubkey) -> Result<AmmKeys> {
    Ok(AmmKeys {
        amm_pool: *amm_pool,
        amm_target: amm.target_orders,
        amm_coin_vault: amm.coin_vault,
        amm_pc_vault: amm.pc_vault,
        amm_lp_mint: amm.lp_mint,
        amm_open_order: amm.open_orders,
        amm_coin_mint: amm.coin_vault_mint,
        amm_pc_mint: amm.pc_vault_mint,
        amm_authority: authority_id(amm_program, AUTHORITY_AMM, amm.nonce as u8)?,
        market: amm.market,
        market_program: amm.market_program,
        nonce: amm.nonce as u8,
    })
}

pub fn calc_total_without_take_pnl_no_orderbook<'a>(
    pc_amount: u64,
    coin_amount: u64,
    amm: &'a AmmInfo,
) -> Result<(u64, u64)> {
    let total_pc_without_take_pnl = pc_amount
        .checked_sub(amm.state_data.need_take_pnl_pc)
        .ok_or(anyhow!("CheckedSubOverflow"))?;
    let total_coin_without_take_pnl = coin_amount
        .checked_sub(amm.state_data.need_take_pnl_coin)
        .ok_or(anyhow!("CheckedSubOverflow"))?;
    Ok((total_pc_without_take_pnl, total_coin_without_take_pnl))
}

pub fn swap_with_slippage(
    pc_vault_amount: u64,
    coin_vault_amount: u64,
    swap_fee_numerator: u64,
    swap_fee_denominator: u64,
    swap_direction: SwapDirection,
    amount_specified: u64,
    swap_base_in: bool,
    slippage_bps: u64,
) -> Result<u64> {
    let other_amount_threshold = swap_exact_amount(
        pc_vault_amount,
        coin_vault_amount,
        swap_fee_numerator,
        swap_fee_denominator,
        swap_direction,
        amount_specified,
        swap_base_in,
    )?;
    let other_amount_threshold = if swap_base_in {
        // min out
        amount_with_slippage(other_amount_threshold, slippage_bps, false)?
    } else {
        // max in
        amount_with_slippage(other_amount_threshold, slippage_bps, true)?
    };
    Ok(other_amount_threshold)
}

pub fn authority_id(program_id: &Pubkey, amm_seed: &[u8], nonce: u8) -> Result<Pubkey> {
    Pubkey::create_program_address(&[amm_seed, &[nonce]], program_id)
        .map_err(|_| anyhow!("InvalidProgramAddress"))
}

pub fn amount_with_slippage(amount: u64, slippage_bps: u64, up_towards: bool) -> Result<u64> {
    let amount = amount;
    println!("real amount {:?}", amount);
    let ten_thounsand = 10000u64;
    let slippage_bps = slippage_bps;
    let amount_with_slippage = if up_towards {
        amount
            .checked_mul(slippage_bps.checked_add(ten_thounsand).unwrap())
            .unwrap()
            .checked_div(ten_thounsand)
            .unwrap()
    } else {
        amount
            .checked_mul(ten_thounsand.checked_sub(slippage_bps).unwrap())
            .unwrap()
            .checked_div(ten_thounsand)
            .unwrap()
    };
    u64::try_from(amount_with_slippage)
        .map_err(|_| anyhow!("failed to read keypair from {}", amount_with_slippage))
}

fn swap_exact_amount(
    pc_vault_amount: u64,
    coin_vault_amount: u64,
    swap_fee_numerator: u64,
    swap_fee_denominator: u64,
    swap_direction: SwapDirection,
    amount_specified: u64,
    swap_base_in: bool,
) -> Result<u64> {
    let other_amount_threshold = if swap_base_in {
        let swap_fee = u128::from(amount_specified)
            .checked_mul(swap_fee_numerator.into())
            .unwrap()
            .checked_div(swap_fee_denominator.into())
            .unwrap();

        let swap_in_after_deduct_fee = u128::from(amount_specified).checked_sub(swap_fee).unwrap();
        let swap_amount_out = swap_token_amount_base_in(
            swap_in_after_deduct_fee,
            pc_vault_amount.into(),
            coin_vault_amount.into(),
            swap_direction,
        ) as u64;
        swap_amount_out
    } else {
        let swap_in_before_add_fee = swap_token_amount_base_out(
            amount_specified.into(),
            pc_vault_amount.into(),
            coin_vault_amount.into(),
            swap_direction,
        );
        let swap_in_after_add_fee = swap_in_before_add_fee
            .checked_mul(swap_fee_denominator.into())
            .unwrap()
            .checked_div(
                (swap_fee_denominator
                    .checked_sub(swap_fee_numerator)
                    .unwrap())
                .into(),
            )
            .unwrap() as u64;

        swap_in_after_add_fee
    };

    Ok(other_amount_threshold)
}

pub fn swap_token_amount_base_in(
    amount_in: u128,
    total_pc_without_take_pnl: u128,
    total_coin_without_take_pnl: u128,
    swap_direction: SwapDirection,
) -> u128 {
    let amount_out;
    match swap_direction {
        SwapDirection::Buy => {
            // (x + delta_x) * (y + delta_y) = x * y
            // (coin + amount_in) * (pc - amount_out) = coin * pc
            // => amount_out = pc - coin * pc / (coin + amount_in)
            // => amount_out = ((pc * coin + pc * amount_in) - coin * pc) / (coin + amount_in)
            // => amount_out =  pc * amount_in / (coin + amount_in)
            let denominator = total_coin_without_take_pnl.checked_add(amount_in).unwrap();
            amount_out = total_pc_without_take_pnl
                .checked_mul(amount_in)
                .unwrap()
                .checked_div(denominator)
                .unwrap();
        }
        SwapDirection::Sell => {
            // (x + delta_x) * (y + delta_y) = x * y
            // (pc + amount_in) * (coin - amount_out) = coin * pc
            // => amount_out = coin - coin * pc / (pc + amount_in)
            // => amount_out = (coin * pc + coin * amount_in - coin * pc) / (pc + amount_in)
            // => amount_out = coin * amount_in / (pc + amount_in)
            let denominator = total_pc_without_take_pnl.checked_add(amount_in).unwrap();
            amount_out = total_coin_without_take_pnl
                .checked_mul(amount_in)
                .unwrap()
                .checked_div(denominator)
                .unwrap();
        }
    }
    return amount_out;
}

pub fn swap_token_amount_base_out(
    amount_out: u128,
    total_pc_without_take_pnl: u128,
    total_coin_without_take_pnl: u128,
    swap_direction: SwapDirection,
) -> u128 {
    let amount_in;
    match swap_direction {
        SwapDirection::Buy => {
            // (x + delta_x) * (y + delta_y) = x * y
            // (coin + amount_in) * (pc - amount_out) = coin * pc
            // => amount_in = coin * pc / (pc - amount_out) - coin
            // => amount_in = (coin * pc - pc * coin + amount_out * coin) / (pc - amount_out)
            // => amount_in = (amount_out * coin) / (pc - amount_out)
            let denominator = total_pc_without_take_pnl.checked_sub(amount_out).unwrap();
            amount_in = total_coin_without_take_pnl
                .checked_mul(amount_out)
                .unwrap()
                .checked_div(denominator)
                .unwrap();
        }
        SwapDirection::Sell => {
            // (x + delta_x) * (y + delta_y) = x * y
            // (pc + amount_in) * (coin - amount_out) = coin * pc
            // => amount_out = coin - coin * pc / (pc + amount_in)
            // => amount_out = (coin * pc + coin * amount_in - coin * pc) / (pc + amount_in)
            // => amount_out = coin * amount_in / (pc + amount_in)

            // => amount_in = coin * pc / (coin - amount_out) - pc
            // => amount_in = (coin * pc - pc * coin + pc * amount_out) / (coin - amount_out)
            // => amount_in = (pc * amount_out) / (coin - amount_out)
            let denominator = total_coin_without_take_pnl.checked_sub(amount_out).unwrap();
            amount_in = total_pc_without_take_pnl
                .checked_mul(amount_out)
                .unwrap()
                .checked_div(denominator)
                .unwrap();
        }
    }
    return amount_in;
}
