use cosmwasm_std::{StdError, StdResult, Storage};
use cw_storage_plus::{Item, Map};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::Asset;

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

pub const DISTRIBUTION_STATUS: Item<DistributionStatus> = Item::new("reservations");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct DistributionStatus {
    pub withdraw_count: u64,
    pub sale_count: u64,
    pub valid_reservations_count: u64,
    pub total_reservation_count: u64,
    pub nft_limit: u64,
}

pub const RESERVATIONS: Map<&[u8], Reservation> = Map::new("reservations");
pub const VALID_RESERVATIONS: Map<&[u8], bool> = Map::new("unfilled_reservations");
pub const RESERVATIONS_BY_ADDRESS: Map<&[u8], Vec<u64>> = Map::new("reservations_by_address");

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct Reservation {
    pub owner: String,
    pub valid: bool,
    pub refundable_at: u64,
}

pub fn invalidate_reservation(storage: &mut dyn Storage, reservation_id: u64) -> StdResult<()> {
    VALID_RESERVATIONS.remove(storage, &reservation_id.to_be_bytes());
    let mut reservation = RESERVATIONS.load(storage, &reservation_id.to_be_bytes())?;
    if !reservation.valid {
        return Err(StdError::generic_err("reservation is invalid"));
    }
    reservation.valid = false;
    RESERVATIONS.save(storage, &reservation_id.to_be_bytes(), &reservation)?;

    let mut distribution_status = DISTRIBUTION_STATUS.load(storage)?;
    distribution_status.valid_reservations_count -= 1;
    DISTRIBUTION_STATUS.save(storage, &distribution_status)
}
