use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::Asset;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    // owner of this distribution contract
    // this is the address that will convert reservations to NFTs
    pub owner: String,
    // cost per nft
    pub cost: Asset,

    // nft contract address this distribution contract is authorized to mint to
    pub nft_contract: Option<String>,
    // maximum number of reservations per address
    pub limit_per_address: u64,
    // maximum number of nfts this contract is allowed to mint
    // so owner cannot dilute the supply
    pub nft_limit: u64,
    // cost can be refunded if NFT is not sent within response_seconds
    pub response_seconds: u64,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    // set the NFT contract address
    // this can only happen once
    SetNftContract {
        contract_addr: String,
    },

    // create a reservation, and pay cost
    ReserveNft {},

    // request a refund given a reservation ID, requires:
    // 1) the message sender be the owner of that reservation
    // 2) the reservation is valid (has not been refunded before, user has not received NFT)
    // 3) it has been response_seconds since reservation has been created and an
    //    nft has not yet been received
    RefundNft {
        reservation_id: u64,
    },

    // the owner of this contract can take a reservation id and convert it to an nft,
    // sending the owner of the reservation an nft in the process
    MintNft {
        reservation_id: u64,
        /// Unique ID of the NFT
        token_id: String,
        /// Identifies the asset to which this NFT represents
        name: String,
        /// Describes the asset to which this NFT represents (may be empty)
        description: Option<String>,
        /// A URI pointing to an image representing the asset
        image: Option<String>,
    },

    // withdraw proceeds from selling nfts. note that this will always leave
    // enough funds in the contract to refund all outstanding reservations
    WithdrawSales {},
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {}
