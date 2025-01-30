use std::{env, time::Duration};

use anyhow::Result;
use regex::Regex;
use solana_sdk::transaction::Transaction;
use time::OffsetDateTime;
use twitter_v2::{
    authorization::BearerToken, id::IntoNumericId, query::TweetField, Authorization, Tweet,
    TwitterApi,
};

use crate::strategy::Strategy;

// 获取用户tweet
pub async fn get_post_content<A: Authorization>(
    api: &TwitterApi<A>,
    id: impl IntoNumericId,
) -> Result<Vec<Tweet>> {
    let res = api
        .get_user_tweets(id)
        .end_time(OffsetDateTime::now_utc())
        .tweet_fields([TweetField::AuthorId, TweetField::CreatedAt])
        .send()
        .await?
        .into_data()
        .expect("get user post error {:?}");
    Ok(res)
}

pub fn auth_for_twitter() -> BearerToken {
    BearerToken::new(std::env::var("APP_BEARER_TOKEN").unwrap())
}

pub async fn process_tweet(tweet: Tweet, strategy: &Strategy) -> Option<Transaction> {
    // fetch the coin name,mint address and gmgn info
    let re = Regex::new(r"[1-9A-HJ-NP-Za-km-z]{32,44}").unwrap();
    if let Some(captures) = re.find(&tweet.text) {
        let mint_address = captures.as_str().to_string();
        // fetch from gmgn,and create a tx
        fetch_coin_info_and_creat_tx(mint_address, env::var("GMGN_COOKIE").unwrap(), strategy).await
    } else {
        return None;
    }
}

pub async fn fetch_coin_info_and_creat_tx(
    mint_address: String,
    cookie: String,
    strategy: &Strategy,
) -> Option<Transaction> {
    // 1. analyze is potenial
    // 2. create a transaction with strategy
    Some(Transaction::default())
}
