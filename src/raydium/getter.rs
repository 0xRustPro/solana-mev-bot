use std::{str::FromStr, sync::Arc};

use crate::new_client;

use super::structure::AmmInfo;

use anyhow::Result;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::{program_pack::Pack, pubkey::Pubkey, signature::Keypair};
use spl_token::state::{Account, Mint};
use spl_token_client::{
    client::{ProgramClient, ProgramRpcClient, ProgramRpcClientSendTransaction},
    token::{TokenError, TokenResult},
};

pub async fn get_multiple_accounts(
    client: Arc<RpcClient>,
    pubkeys: &[Pubkey],
) -> Result<Vec<Option<solana_sdk::account::Account>>> {
    let res = client.get_multiple_accounts(pubkeys).await?;
    Ok(res)
}

pub async fn get_account_info(
    client: Arc<RpcClient>,
    _keypair: Arc<Keypair>,
    address: &Pubkey,
    account: &Pubkey,
) -> TokenResult<Account> {
    match client.get_account(account).await {
        Ok(account) => {
            if account.owner != spl_token::ID {
                return Err(TokenError::AccountInvalidOwner);
            }

            let account = Account::unpack(&account.data)?;
            if account.mint != *address {
                return Err(TokenError::AccountInvalidMint);
            }

            Ok(account)
        }
        Err(_) => {
            return Err(TokenError::AccountNotFound);
        }
    }
}

pub async fn get_mint_info(
    client: Arc<RpcClient>,
    _keypair: Arc<Keypair>,
    address: &Pubkey,
) -> TokenResult<Mint> {
    let program_client = Arc::new(ProgramRpcClient::new(
        client.clone(),
        ProgramRpcClientSendTransaction,
    ));
    let account = program_client
        .get_account(*address)
        .await
        .map_err(TokenError::Client)?
        .ok_or(TokenError::AccountNotFound)
        .inspect_err(|err| tracing::warn!("{} {}: mint {}", address, err, address))?;

    if account.owner != spl_token::ID {
        return Err(TokenError::AccountInvalidOwner);
    }

    let mint_result = Mint::unpack(&account.data).map_err(Into::into);
    let decimals: Option<u8> = None;
    if let (Ok(mint), Some(decimals)) = (&mint_result, decimals) {
        if decimals != mint.decimals {
            return Err(TokenError::InvalidDecimals);
        }
    }

    mint_result
}

// 通过池子id获取池子当前信息
pub async fn get_pool_state(client: Arc<RpcClient>, pool_id: &str) -> Result<(Pubkey, AmmInfo)> {
    let amm_pool_id = Pubkey::from_str(pool_id)?;

    // 获取账户信息
    let account_data = get_account(client.clone(), &amm_pool_id).await?.unwrap();

    // 转换为amm_info
    let amm_state = AmmInfo::load_from_bytes(&account_data).unwrap();
    Ok((amm_pool_id, amm_state.clone()))
}

// 获取账户信息
pub async fn get_account(client: Arc<RpcClient>, addr: &Pubkey) -> Result<Option<Vec<u8>>> {
    if let Some(account) = client
        .get_account_with_commitment(
            addr,
            solana_sdk::commitment_config::CommitmentConfig::processed(),
        )
        .await?
        .value
    {
        let account_data = account.data;
        Ok(Some(account_data))
    } else {
        Ok(None)
    }
}

#[tokio::test]
async fn test_get_pool_state() -> Result<()> {
    let client = new_client();
    get_pool_state(client, "3gfdqZ2DqFwYufzy1G49evXcXkmtWfUj4tfmg8zUg6zB").await?;
    Ok(())
}
