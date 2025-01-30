use anyhow::{anyhow, Ok, Result};
use borsh::{BorshDeserialize, BorshSerialize};
use reqwest::multipart::{Form, Part};
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use std::{fs::File, io::Read, sync::Arc};

use crate::constants;

use super::accounts::{BondingCurveAccount, GlobalAccount};

/// 获取bonding curve
pub fn get_bonding_curve_pda(mint: &Pubkey) -> Option<Pubkey> {
    let seeds: &[&[u8]; 2] = &[constants::seeds::BONDING_CURVE_SEED, mint.as_ref()];
    let program_id: &Pubkey = &constants::accounts::PUMPFUN;
    let pda: Option<(Pubkey, u8)> = Pubkey::try_find_program_address(seeds, program_id);
    pda.map(|pubkey| pubkey.0)
}

/// 获取bonding curve的账户封装
pub async fn get_bonding_curve_account(
    client: Arc<RpcClient>,
    mint: &Pubkey,
) -> Result<BondingCurveAccount> {
    let bonding_curve_pda = get_bonding_curve_pda(mint).ok_or(anyhow!("BondingCurveNotFound"))?;

    let account = client
        .get_account(&bonding_curve_pda)
        .await
        .map_err(|_keypair| anyhow!("SolanaClientError"))?;

    BondingCurveAccount::try_from_slice(&account.data).map_err(|_| anyhow!("BorshError"))
}

/// 获取global program地址
pub fn get_global_pda() -> Pubkey {
    let seeds: &[&[u8]; 1] = &[constants::seeds::GLOBAL_SEED];
    let program_id: &Pubkey = &constants::accounts::PUMPFUN;
    Pubkey::find_program_address(seeds, program_id).0
}

/// 获取global program的账户封装
pub async fn get_global_account(client: Arc<RpcClient>) -> Result<GlobalAccount> {
    let global: Pubkey = get_global_pda();

    let account = client
        .get_account(&global)
        .await
        .map_err(|_| anyhow!("SolanaClientError"))?;

    GlobalAccount::try_from_slice(&account.data).map_err(|e| anyhow!("BorshError"))
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct CreateTokenMetadata {
    name: String,
    symbol: String,
    description: String,
    twitter: Option<String>,
    telegram: Option<String>,
    website: Option<String>,
    show_name: bool,
    file: String,
}

pub async fn create_token_meta_data(create_meta_data: CreateTokenMetadata) -> Result<String> {
    let mut file = File::open(create_meta_data.file)?;
    let mut file_content = Vec::new();
    file.read_to_end(&mut file_content)?;

    let mut form = Form::new()
        .text("name", create_meta_data.name)
        .text("symbol", create_meta_data.symbol)
        .text("description", create_meta_data.description)
        .text("showName", create_meta_data.show_name.to_string())
        .part(
            "file",
            Part::bytes(file_content)
                .file_name("file")
                .mime_str("image/png")?,
        );
    if create_meta_data.twitter.is_some() {
        form = form.text("twitter", create_meta_data.twitter.unwrap());
    }
    if create_meta_data.telegram.is_some() {
        form = form.text("telegram", create_meta_data.telegram.unwrap());
    }
    if create_meta_data.website.is_some() {
        form = form.text("website", create_meta_data.website.unwrap());
    }
    println!("{:?}", form);
    let client = reqwest::Client::new();

    // 发送 POST 请求到 IPFS 接口
    let metadata_response = client
        .post("https://pump.fun/api/ipfs")
        .multipart(form)
        .send()
        .await?;
    let metadata_response_json = metadata_response.text().await?;
    println!("Metadata URI: {}", metadata_response_json);
    Ok(metadata_response_json)
}

#[tokio::test]
async fn test_create_token_metadata() {
    // Create a temporary file
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join("test_image.png");
    std::fs::write(&file_path, b"fake image data").unwrap();
    println!("{:?}", true.to_string());
    // Create test metadata
    let metadata = CreateTokenMetadata {
        name: "Test Token".to_string(),
        symbol: "TEST".to_string(),
        description: "Test Description".to_string(),
        file: file_path.to_str().unwrap().to_string(),
        twitter: None,
        telegram: None,
        website: None,
        show_name: true,
    };

    // Call the function
    let result = create_token_meta_data(metadata).await.unwrap();
}
