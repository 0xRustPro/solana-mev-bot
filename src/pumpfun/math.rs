use anyhow::{anyhow, Result};
pub fn amount_with_slippage(amount: u64, slippage_bps: u64, is_buy: bool) -> Result<u64> {
    let amount = amount;
    println!("real amount {:?}", amount);
    let ten_thounsand = 10000u64;
    let slippage_bps = slippage_bps;
    // buy false
    let amount_with_slippage = if is_buy {
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
