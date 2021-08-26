use cosmwasm_std::{StdResult, Storage};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::{Asset};

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
    pub cost: Asset,
    pub nft_contract: Option<String>,
    pub limit_per_address: u64,
    pub nft_limit: u64,
    pub response_seconds: u64,
}

pub const SALE_COUNT: Item<u64> = Item::new("sale_count");
pub const WITHDRAW_COUNT: Item<u64> = Item::new("withdraw_count");

pub const RESERVATIONS: Map<&[u8], Reservation> = Map::new("reservations");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Reservation {
    pub owner: String,
    pub valid: bool,
    pub refundable_at: u64,
}

pub const RESERVATION_COUNT: Map<&[u8], u64> = Map::new("reservation_count");
