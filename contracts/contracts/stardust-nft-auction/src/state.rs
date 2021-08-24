use cosmwasm_std::{Decimal, StdResult, Storage, Uint128};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::{Asset, AssetInfo};

pub const UNIQUE_ID: Item<u64> = Item::new("unique_id");

pub fn new_id(storage: &mut dyn Storage) -> StdResult<u64> {
    let cur = UNIQUE_ID.load(storage)?;
    UNIQUE_ID.save(storage, &(cur + 1))?;
    Ok(cur)
}

pub const CONFIG: Item<Config> = Item::new("config");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Config {
    pub owner: String,
    pub fee: Decimal,
}

// (auction_id) -> auction
pub const AUCTIONS: Map<&[u8], Auction> = Map::new("auctions");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Auction {
    pub nft_contract: String,
    pub token_id: String,
    pub creator: String,
    pub min_bid: Uint128,
    pub top_bidder: Option<String>,
    pub bid_amount: Uint128,
    pub expiration_date: u64,
    pub denom: AssetInfo,
    pub closed: bool,
}

// (auction_id, address) -> bid
pub const AUCTION_BIDS: Map<(&[u8], &[u8]), Asset> = Map::new("auction_bids");

// (order_id) -> sell order
pub const SELL_ORDERS: Map<&[u8], SellOrder> = Map::new("sell_orders");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct SellOrder {
    pub creator: String,
    pub nft_contract: String,
    pub token_id: String,
    pub requested_asset: Asset,
    pub cancelled: bool,
}

pub const FEE_STORAGE: Map<&[u8], Uint128> = Map::new("fee_storage");
