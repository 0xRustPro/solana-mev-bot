use anyhow::Result;
use bytemuck::AnyBitPattern;
use serde::Deserialize;
use solana_sdk::pubkey::Pubkey;

#[derive(Clone, Copy, Debug, Default, PartialEq, AnyBitPattern)]
#[repr(C, packed)]
pub struct AmmInfo {
    /// 初始化状态。
    pub status: u64,
    /// 用于生成程序地址的 nonce。
    /// 程序地址是通过 nonce、amm 程序 ID 和 amm 账户公钥确定性生成的。
    /// 该程序地址对 amm 的代币 coin 账户、代币 pc 账户和池子代币 mint 拥有权限。
    pub nonce: u64,
    /// 最大订单数量。
    pub order_num: u64,
    /// 价格深度范围，5 表示 5% 的范围。
    pub depth: u64,
    /// coin 代币的小数位数。
    pub coin_decimals: u64,
    /// pc 代币的小数位数。
    pub pc_decimals: u64,
    /// amm 的机器状态。
    pub state: u64,
    /// amm 的重置标志。
    pub reset_flag: u64,
    /// 最小交易量，1 表示 0.000001。
    pub min_size: u64,
    /// 最大交易量削减比例分子，分母为 sys_decimal_value。
    pub vol_max_cut_ratio: u64,
    /// 交易量波动分子，分母为 sys_decimal_value。
    pub amount_wave: u64,
    /// coin 的最小交易单位，1 表示 0.000001。
    pub coin_lot_size: u64,
    /// pc 的最小交易单位，1 表示 0.000001。
    pub pc_lot_size: u64,
    /// 最小当前价格乘数：(2 * amm.order_num * amm.pc_lot_size) * max_price_multiplier。
    pub min_price_multiplier: u64,
    /// 最大当前价格乘数：(2 * amm.order_num * amm.pc_lot_size) * max_price_multiplier。
    pub max_price_multiplier: u64,
    /// 系统小数位值，用于标准化 coin 和 pc 的数量。
    pub sys_decimal_value: u64,
    /// 所有费用信息。
    pub fees: Fees,
    /// 统计数据。
    pub state_data: StateData,
    /// coin 代币的保险库地址。
    pub coin_vault: Pubkey,
    /// pc 代币的保险库地址。
    pub pc_vault: Pubkey,
    /// coin 代币的 mint 地址。
    /// 注意：coin代币
    pub coin_vault_mint: Pubkey,
    /// pc 代币的 mint 地址。
    pub pc_vault_mint: Pubkey,
    /// 流动性提供者（LP）代币的 mint 地址。
    pub lp_mint: Pubkey,
    /// 开放订单的地址。
    pub open_orders: Pubkey,
    /// 市场的地址。
    pub market: Pubkey,
    /// 市场程序的地址。
    pub market_program: Pubkey,
    /// 目标订单的地址。
    pub target_orders: Pubkey,
    /// 填充字段 1。
    pub padding1: [u64; 8],
    /// amm 所有者的地址。
    pub amm_owner: Pubkey,
    /// 池子中 LP 代币的数量。
    pub lp_amount: u64,
    /// 客户端订单 ID。
    pub client_order_id: u64,
    /// 最近的 epoch。
    pub recent_epoch: u64,
    /// 填充字段 2。
    pub padding2: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, AnyBitPattern)]
#[repr(C, packed)]
pub struct StateData {
    /// delay to take pnl coin
    pub need_take_pnl_coin: u64,
    /// delay to take pnl pc
    pub need_take_pnl_pc: u64,
    /// total pnl pc
    pub total_pnl_pc: u64,
    /// total pnl coin
    pub total_pnl_coin: u64,
    /// ido pool open time
    pub pool_open_time: u64,
    /// padding for future updates
    pub padding: [u64; 2],
    /// switch from orderbookonly to init
    pub orderbook_to_init_time: u64,

    /// swap coin in amount
    pub swap_coin_in_amount: u128,
    /// swap pc out amount
    pub swap_pc_out_amount: u128,
    /// charge pc as swap fee while swap pc to coin
    pub swap_acc_pc_fee: u64,

    /// swap pc in amount
    pub swap_pc_in_amount: u128,
    /// swap coin out amount
    pub swap_coin_out_amount: u128,
    /// charge coin as swap fee while swap coin to pc
    pub swap_acc_coin_fee: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AmmSwapInfoResult {
    pub pool_id: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_open_orders: Pubkey,
    pub amm_coin_vault: Pubkey,
    pub amm_pc_vault: Pubkey,
    pub input_mint: Pubkey,
    pub output_mint: Pubkey,
    pub market_program: Pubkey,
    pub market: Pubkey,
    pub market_coin_vault: Pubkey,
    pub market_pc_vault: Pubkey,
    pub market_vault_signer: Pubkey,
    pub market_event_queue: Pubkey,
    pub market_bids: Pubkey,
    pub market_asks: Pubkey,
    pub amount_specified: u64,
    pub other_amount_threshold: u64,
}

#[derive(Clone, Copy, Debug)]
pub struct AmmKeys {
    pub amm_pool: Pubkey,
    pub amm_coin_mint: Pubkey,
    pub amm_pc_mint: Pubkey,
    pub amm_authority: Pubkey,
    pub amm_target: Pubkey,
    pub amm_coin_vault: Pubkey,
    pub amm_pc_vault: Pubkey,
    pub amm_lp_mint: Pubkey,
    pub amm_open_order: Pubkey,
    pub market_program: Pubkey,
    pub market: Pubkey,
    pub nonce: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub enum SwapDirection {
    #[serde(rename = "buy")]
    Buy,
    #[serde(rename = "sell")]
    Sell,
}

#[repr(u64)]
pub enum AmmStatus {
    Uninitialized = 0u64,
    Initialized = 1u64,
    Disabled = 2u64,
    WithdrawOnly = 3u64,
    // pool only can add or remove liquidity, can't swap and plan orders
    LiquidityOnly = 4u64,
    // pool only can add or remove liquidity and plan orders, can't swap
    OrderBookOnly = 5u64,
    // pool only can add or remove liquidity and swap, can't plan orders
    SwapOnly = 6u64,
    // pool status after created and will auto update to SwapOnly during swap after open_time
    WaitingTrade = 7u64,
}

impl AmmStatus {
    pub fn from_u64(status: u64) -> Self {
        match status {
            0u64 => AmmStatus::Uninitialized,
            1u64 => AmmStatus::Initialized,
            2u64 => AmmStatus::Disabled,
            3u64 => AmmStatus::WithdrawOnly,
            4u64 => AmmStatus::LiquidityOnly,
            5u64 => AmmStatus::OrderBookOnly,
            6u64 => AmmStatus::SwapOnly,
            7u64 => AmmStatus::WaitingTrade,
            _ => unreachable!(),
        }
    }

    pub fn into_u64(&self) -> u64 {
        match self {
            AmmStatus::Uninitialized => 0u64,
            AmmStatus::Initialized => 1u64,
            AmmStatus::Disabled => 2u64,
            AmmStatus::WithdrawOnly => 3u64,
            AmmStatus::LiquidityOnly => 4u64,
            AmmStatus::OrderBookOnly => 5u64,
            AmmStatus::SwapOnly => 6u64,
            AmmStatus::WaitingTrade => 7u64,
        }
    }
    pub fn valid_status(status: u64) -> bool {
        match status {
            1u64 | 2u64 | 3u64 | 4u64 | 5u64 | 6u64 | 7u64 => return true,
            _ => return false,
        }
    }

    pub fn deposit_permission(&self) -> bool {
        match self {
            AmmStatus::Uninitialized => false,
            AmmStatus::Initialized => true,
            AmmStatus::Disabled => false,
            AmmStatus::WithdrawOnly => false,
            AmmStatus::LiquidityOnly => true,
            AmmStatus::OrderBookOnly => true,
            AmmStatus::SwapOnly => true,
            AmmStatus::WaitingTrade => true,
        }
    }

    pub fn withdraw_permission(&self) -> bool {
        match self {
            AmmStatus::Uninitialized => false,
            AmmStatus::Initialized => true,
            AmmStatus::Disabled => false,
            AmmStatus::WithdrawOnly => true,
            AmmStatus::LiquidityOnly => true,
            AmmStatus::OrderBookOnly => true,
            AmmStatus::SwapOnly => true,
            AmmStatus::WaitingTrade => true,
        }
    }

    pub fn swap_permission(&self) -> bool {
        match self {
            AmmStatus::Uninitialized => false,
            AmmStatus::Initialized => true,
            AmmStatus::Disabled => false,
            AmmStatus::WithdrawOnly => false,
            AmmStatus::LiquidityOnly => false,
            AmmStatus::OrderBookOnly => false,
            AmmStatus::SwapOnly => true,
            AmmStatus::WaitingTrade => true,
        }
    }

    pub fn orderbook_permission(&self) -> bool {
        match self {
            AmmStatus::Uninitialized => false,
            AmmStatus::Initialized => true,
            AmmStatus::Disabled => false,
            AmmStatus::WithdrawOnly => false,
            AmmStatus::LiquidityOnly => false,
            AmmStatus::OrderBookOnly => true,
            AmmStatus::SwapOnly => false,
            AmmStatus::WaitingTrade => false,
        }
    }
}

#[derive(Copy, Clone)]
#[repr(u64)]
pub enum AmmParams {
    Status = 0u64,
    State = 1u64,
    OrderNum = 2u64,
    Depth = 3u64,
    AmountWave = 4u64,
    MinPriceMultiplier = 5u64,
    MaxPriceMultiplier = 6u64,
    MinSize = 7u64,
    VolMaxCutRatio = 8u64,
    Fees = 9u64,
    AmmOwner = 10u64,
    SetOpenTime = 11u64,
    LastOrderDistance = 12u64,
    InitOrderDepth = 13u64,
    SetSwitchTime = 14u64,
    ClearOpenTime = 15u64,
    Seperate = 16u64,
    UpdateOpenOrder = 17u64,
}

impl AmmParams {
    pub fn from_u64(state: u64) -> Self {
        match state {
            0u64 => AmmParams::Status,
            1u64 => AmmParams::State,
            2u64 => AmmParams::OrderNum,
            3u64 => AmmParams::Depth,
            4u64 => AmmParams::AmountWave,
            5u64 => AmmParams::MinPriceMultiplier,
            6u64 => AmmParams::MaxPriceMultiplier,
            7u64 => AmmParams::MinSize,
            8u64 => AmmParams::VolMaxCutRatio,
            9u64 => AmmParams::Fees,
            10u64 => AmmParams::AmmOwner,
            11u64 => AmmParams::SetOpenTime,
            12u64 => AmmParams::LastOrderDistance,
            13u64 => AmmParams::InitOrderDepth,
            14u64 => AmmParams::SetSwitchTime,
            15u64 => AmmParams::ClearOpenTime,
            16u64 => AmmParams::Seperate,
            17u64 => AmmParams::UpdateOpenOrder,
            _ => unreachable!(),
        }
    }

    pub fn into_u64(&self) -> u64 {
        match self {
            AmmParams::Status => 0u64,
            AmmParams::State => 1u64,
            AmmParams::OrderNum => 2u64,
            AmmParams::Depth => 3u64,
            AmmParams::AmountWave => 4u64,
            AmmParams::MinPriceMultiplier => 5u64,
            AmmParams::MaxPriceMultiplier => 6u64,
            AmmParams::MinSize => 7u64,
            AmmParams::VolMaxCutRatio => 8u64,
            AmmParams::Fees => 9u64,
            AmmParams::AmmOwner => 10u64,
            AmmParams::SetOpenTime => 11u64,
            AmmParams::LastOrderDistance => 12u64,
            AmmParams::InitOrderDepth => 13u64,
            AmmParams::SetSwitchTime => 14u64,
            AmmParams::ClearOpenTime => 15u64,
            AmmParams::Seperate => 16u64,
            AmmParams::UpdateOpenOrder => 17u64,
        }
    }
}

impl AmmInfo {
    pub fn load_from_bytes(data: &[u8]) -> Result<&Self> {
        Ok(bytemuck::from_bytes(data))
    }
}

use anyhow::anyhow;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_sdk::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
};
#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, AnyBitPattern)]
pub struct Fees {
    /// numerator of the min_separate
    pub min_separate_numerator: u64,
    /// denominator of the min_separate
    pub min_separate_denominator: u64,

    /// numerator of the fee
    pub trade_fee_numerator: u64,
    /// denominator of the fee
    /// and 'trade_fee_denominator' must be equal to 'min_separate_denominator'
    pub trade_fee_denominator: u64,

    /// numerator of the pnl
    pub pnl_numerator: u64,
    /// denominator of the pnl
    pub pnl_denominator: u64,

    /// numerator of the swap_fee
    pub swap_fee_numerator: u64,
    /// denominator of the swap_fee
    pub swap_fee_denominator: u64,
}
impl Fees {
    /// Validate that the fees are reasonable
    pub fn validate(&self) -> Result<()> {
        validate_fraction(self.min_separate_numerator, self.min_separate_denominator)?;
        validate_fraction(self.trade_fee_numerator, self.trade_fee_denominator)?;
        validate_fraction(self.pnl_numerator, self.pnl_denominator)?;
        validate_fraction(self.swap_fee_numerator, self.swap_fee_denominator)?;
        Ok(())
    }

    pub fn initialize(&mut self) -> Result<()> {
        // min_separate = 5/10000
        self.min_separate_numerator = 5;
        self.min_separate_denominator = 10000;
        // trade_fee = 25/10000
        self.trade_fee_numerator = 25;
        self.trade_fee_denominator = 10000;
        // pnl = 12/100
        self.pnl_numerator = 12;
        self.pnl_denominator = 100;
        // swap_fee = 25 / 10000
        self.swap_fee_numerator = 25;
        self.swap_fee_denominator = 10000;
        Ok(())
    }
}

/// IsInitialized is required to use `Pack::pack` and `Pack::unpack`
impl IsInitialized for Fees {
    fn is_initialized(&self) -> bool {
        true
    }
}

impl Sealed for Fees {}
impl Pack for Fees {
    const LEN: usize = 64;
    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, 64];
        let (
            min_separate_numerator,
            min_separate_denominator,
            trade_fee_numerator,
            trade_fee_denominator,
            pnl_numerator,
            pnl_denominator,
            swap_fee_numerator,
            swap_fee_denominator,
        ) = mut_array_refs![output, 8, 8, 8, 8, 8, 8, 8, 8];
        *min_separate_numerator = self.min_separate_numerator.to_le_bytes();
        *min_separate_denominator = self.min_separate_denominator.to_le_bytes();
        *trade_fee_numerator = self.trade_fee_numerator.to_le_bytes();
        *trade_fee_denominator = self.trade_fee_denominator.to_le_bytes();
        *pnl_numerator = self.pnl_numerator.to_le_bytes();
        *pnl_denominator = self.pnl_denominator.to_le_bytes();
        *swap_fee_numerator = self.swap_fee_numerator.to_le_bytes();
        *swap_fee_denominator = self.swap_fee_denominator.to_le_bytes();
    }

    fn unpack_from_slice(input: &[u8]) -> Result<Fees, ProgramError> {
        let input = array_ref![input, 0, 64];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            min_separate_numerator,
            min_separate_denominator,
            trade_fee_numerator,
            trade_fee_denominator,
            pnl_numerator,
            pnl_denominator,
            swap_fee_numerator,
            swap_fee_denominator,
        ) = array_refs![input, 8, 8, 8, 8, 8, 8, 8, 8];
        Ok(Self {
            min_separate_numerator: u64::from_le_bytes(*min_separate_numerator),
            min_separate_denominator: u64::from_le_bytes(*min_separate_denominator),
            trade_fee_numerator: u64::from_le_bytes(*trade_fee_numerator),
            trade_fee_denominator: u64::from_le_bytes(*trade_fee_denominator),
            pnl_numerator: u64::from_le_bytes(*pnl_numerator),
            pnl_denominator: u64::from_le_bytes(*pnl_denominator),
            swap_fee_numerator: u64::from_le_bytes(*swap_fee_numerator),
            swap_fee_denominator: u64::from_le_bytes(*swap_fee_denominator),
        })
    }
}
fn validate_fraction(numerator: u64, denominator: u64) -> Result<()> {
    if numerator >= denominator || denominator == 0 {
        Err(anyhow!("InvalidFee"))
    } else {
        Ok(())
    }
}
