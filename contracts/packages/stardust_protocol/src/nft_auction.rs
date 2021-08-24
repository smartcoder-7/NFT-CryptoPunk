use cosmwasm_std::{Decimal, Uint128};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use terraswap::asset::{AssetInfo, Asset};
use cw721::Cw721ReceiveMsg;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
pub struct InstantiateMsg {
    pub owner: String,
    pub fee: Decimal,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ExecuteMsg {
    ReceiveNft(Cw721ReceiveMsg),

    // update owner and fee; callable only by owner
    UpdateConfig {
        owner: String,
        fee: Decimal,
    },

    // increase your bid for a given auction_id
    // if you do not have a bid, it creates a new bid
    IncreaseAuctionBid {
        auction_id: u64,
        asset: Asset
    },

    // withdraw your auction bid and returns your assets
    // callable only if you are not the top bidder
    WithdrawAuctionBid {
        auction_id: u64
    },

    // fill someone's sell order to directly buy an NFT
    FillSellOrder {
        order_id: u64,
    },

    // cancel a sell order for an NFT that you created
    CancelSellOrder {
        order_id: u64
    },

    // close an auction
    // 1) before auction expiration, callable only by auction creator
    //    - if nobody has provided a bid, NFT is returned
    //    - otherwise top bid is taken and NFT is exchanged
    //    - require_no_bidder, makes this call error if there is a ongoing bid
    //      this is so there is no race condition where you think you are closing
    //      an auction you made with no bidders, but someone places a bid in the same block
    // 2) after auction expiration, callable by anyone
    CloseAuction {
        auction_id: u64,
        require_no_bidder: bool
    },

    // withdraw fees (in some asset) collected by contract
    // and sends to arbitrary address
    // callable only by owner (governance?)
    WithdrawFees {
        asset_info: AssetInfo,
        address: String
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Cw721HookMsg {
    // create sell order for some NFT, request some amount of some asset
    CreateSellOrder {
        requested_asset: Asset,
    },

    // create an auction denominated in some asset (possibly aTerra tokens)
    CreateAuction {
        min_bid: Uint128,
        expiration_date: u64,
        denom: AssetInfo,
    },
}


#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum QueryMsg {}
