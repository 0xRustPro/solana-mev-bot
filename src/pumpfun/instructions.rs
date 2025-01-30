use borsh::{BorshDeserialize, BorshSerialize};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signature::Keypair,
    signer::Signer,
};
use spl_associated_token_account::get_associated_token_address;

use crate::constants;

use super::utils::{get_bonding_curve_pda, get_global_pda};

#[derive(BorshSerialize, BorshDeserialize)]
struct BuyArgs {
    amount: u64,
    max_sol_cost: u64,
}
#[derive(BorshSerialize, BorshDeserialize)]
struct SellArgs {
    amount: u64,
    min_sol_output: u64,
}
#[derive(BorshSerialize, BorshDeserialize)]
struct CreateArgs {
    /// Name of the token
    pub name: String,
    /// Token symbol (e.g. "BTC")
    pub symbol: String,
    /// Description of the token
    pub description: String,
    /// Path to the token's image file
    pub file: String,
    /// Optional Twitter handle
    pub twitter: Option<String>,
    /// Optional Telegram group
    pub telegram: Option<String>,
    /// Optional website URL
    pub website: Option<String>,
}

// 指令的标识符
const BUY_INSTRUCTION_DISCRIMINATOR: u8 = 102;
const SELL_INSTRUCTION_DISCRIMINATOR: u8 = 51;

pub fn create_buy_instruction(
    payer: &Keypair,
    mint: &Pubkey,
    amount: u64,
    max_sol_cost: u64,
) -> Instruction {
    let bonding_curve: Pubkey = get_bonding_curve_pda(mint).unwrap();

    // 准备账户列表
    let accounts = vec![
        AccountMeta::new(get_global_pda(), false),
        AccountMeta::new(constants::accounts::PUMPFUN_FEE_RECEIPT, false),
        AccountMeta::new(*mint, false),
        AccountMeta::new(bonding_curve, false),
        AccountMeta::new(get_associated_token_address(&bonding_curve, mint), false),
        AccountMeta::new(get_associated_token_address(&payer.pubkey(), mint), false),
        AccountMeta::new(payer.pubkey(), true),
        AccountMeta::new_readonly(constants::accounts::SYSTEM_PROGRAM, false),
        AccountMeta::new_readonly(constants::accounts::TOKEN_PROGRAM, false),
        AccountMeta::new_readonly(constants::accounts::RENT, false),
        AccountMeta::new_readonly(constants::accounts::EVENT_AUTHORITY, false),
        AccountMeta::new_readonly(constants::accounts::PUMPFUN, false),
    ];

    // 准备指令参数
    let args = BuyArgs {
        amount,
        max_sol_cost,
    };

    // 序列化指令数据
    let mut data = vec![BUY_INSTRUCTION_DISCRIMINATOR];
    args.serialize(&mut data).unwrap();

    // 返回 Instruction
    Instruction {
        program_id: constants::accounts::PUMPFUN,
        accounts,
        data,
    }
}

pub fn create_sell_instruction(
    payer: &Keypair,
    mint: &Pubkey,
    amount: u64,
    min_sol_output: u64,
) -> Instruction {
    let bonding_curve: Pubkey = get_bonding_curve_pda(mint).unwrap();

    let accounts = vec![
        AccountMeta::new(get_global_pda(), false), //gloabl
        AccountMeta::new(constants::accounts::PUMPFUN_FEE_RECEIPT, false), // fee receipient
        AccountMeta::new(*mint, false),            // mint
        AccountMeta::new(bonding_curve, false),    // bonding curve
        AccountMeta::new(get_associated_token_address(&bonding_curve, mint), false), // associated bonding curve
        AccountMeta::new(get_associated_token_address(&payer.pubkey(), mint), false), // associated user
        AccountMeta::new(payer.pubkey(), true),                                       // user
        AccountMeta::new_readonly(constants::accounts::SYSTEM_PROGRAM, false), // system program
        AccountMeta::new_readonly(constants::accounts::TOKEN_PROGRAM, false), // associated token program
        AccountMeta::new_readonly(constants::accounts::RENT, false),          // token program
        AccountMeta::new_readonly(constants::accounts::EVENT_AUTHORITY, false), // event authority
        AccountMeta::new_readonly(constants::accounts::PUMPFUN, false),       // pump fun program
    ];

    let args = SellArgs {
        amount,
        min_sol_output,
    };

    let mut data = vec![SELL_INSTRUCTION_DISCRIMINATOR];
    args.serialize(&mut data).unwrap();
    Instruction {
        program_id: constants::accounts::PUMPFUN,
        accounts,
        data,
    }
}

// pub fn create_token_instruction(payer: &Keypair, mint: &Pubkey) -> Instruction {}
