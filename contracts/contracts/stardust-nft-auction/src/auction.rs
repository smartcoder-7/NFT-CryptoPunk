use crate::state::{new_id, Auction, AUCTIONS, AUCTION_BIDS, CONFIG, FEE_STORAGE};
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, StdResult, Uint128,
    WasmMsg,
};
use cw20_base::msg::ExecuteMsg as TokenMsg;
use cw721::Cw721ExecuteMsg;
use terraswap::asset::{Asset, AssetInfo};

pub fn create_auction(
    deps: DepsMut,
    info: MessageInfo,
    creator: String,
    token_id: String,
    min_bid: Uint128,
    expiration_date: u64,
    denom: AssetInfo,
) -> StdResult<Response> {
    let auction = Auction {
        nft_contract: info.sender.into_string(),
        token_id,
        creator,
        min_bid,
        top_bidder: None,
        bid_amount: Uint128::zero(),
        expiration_date,
        denom,
        closed: false,
    };

    let auction_id = new_id(deps.storage)?;
    AUCTIONS.save(deps.storage, &auction_id.to_be_bytes(), &auction)?;
    Ok(Response::new())
}

pub fn close_auction(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    auction_id: u64,
    require_no_bidder: bool,
) -> StdResult<Response> {
    let mut auction = AUCTIONS.load(deps.storage, &auction_id.to_be_bytes())?;

    if auction.closed {
        return Err(StdError::generic_err("auction is already closed"));
    }

    if require_no_bidder && auction.top_bidder.is_some() {
        return Err(StdError::generic_err("auction has bidder"));
    }

    if env.block.time.seconds() < auction.expiration_date && info.sender != auction.creator {
        return Err(StdError::generic_err("unauthorized"));
    }

    // faciliate nft exchange
    let mut messages = vec![];

    match auction.top_bidder.clone() {
        Some(bidder) => {
            // facilitate exchange w/ top bidder
            // collect fee
            let cfg = CONFIG.load(deps.storage)?;

            let fee_amt = auction.bid_amount * cfg.fee;
            let to_creator = Asset {
                info: auction.denom.clone(),
                amount: auction.bid_amount - fee_amt,
            };

            let mut current_fee =
                FEE_STORAGE.load(deps.storage, auction.denom.to_string().as_bytes())?;
            current_fee += fee_amt;

            FEE_STORAGE.save(
                deps.storage,
                auction.denom.to_string().as_bytes(),
                &current_fee,
            )?;

            messages.push(to_creator.into_msg(&deps.querier, Addr::unchecked(auction.creator.clone()))?);
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: auction.nft_contract.clone(),
                msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: bidder,
                    token_id: auction.token_id.clone(),
                })?,
                funds: vec![],
            }))
        }
        None => {
            // transfer NFT back to creator
            messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: auction.nft_contract.clone(),
                msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                    recipient: auction.creator.clone(),
                    token_id: auction.token_id.clone(),
                })?,
                funds: vec![],
            }))
        }
    }
    auction.closed = true;
    AUCTIONS.save(deps.storage, &auction_id.to_be_bytes(), &auction)?;

    Ok(Response::new().add_messages(messages))
}

pub fn withdraw_auction_bid(
    deps: DepsMut,
    info: MessageInfo,
    auction_id: u64,
) -> StdResult<Response> {
    let auction = AUCTIONS.load(deps.storage, &auction_id.to_be_bytes())?;
    if auction.top_bidder.unwrap_or(String::default()) == info.sender.clone().into_string() {
        return Err(StdError::generic_err("cannot withdraw top bid"));
    }

    let current_bid = AUCTION_BIDS.load(
        deps.storage,
        (&auction_id.to_be_bytes(), &info.sender.as_bytes()),
    )?;

    AUCTION_BIDS.save(
        deps.storage,
        (&auction_id.to_be_bytes(), &info.sender.as_bytes()),
        &Asset {
            info: current_bid.info.clone(),
            amount: Uint128::zero(),
        },
    )?;

    Ok(Response::new().add_message(current_bid.into_msg(&deps.querier, info.sender)?))
}

pub fn increase_auction_bid(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    auction_id: u64,
    asset: Asset,
) -> StdResult<Response> {
    let mut auction = AUCTIONS.load(deps.storage, &auction_id.to_be_bytes())?;
    asset.assert_sent_native_token_balance(&info)?;
    if auction.expiration_date <= env.block.time.seconds() || auction.closed {
        return Err(StdError::generic_err("auction is closed or expired"));
    }

    if auction.denom != asset.info {
        return Err(StdError::generic_err(
            "auction is denominated in a different asset",
        ));
    }

    let mut current_bid = AUCTION_BIDS
        .load(
            deps.storage,
            (&auction_id.to_be_bytes(), info.sender.as_bytes()),
        )
        .unwrap_or(Asset {
            info: asset.info.clone(),
            amount: Uint128::zero(),
        });
    current_bid.amount += asset.amount;
    if current_bid.amount < auction.min_bid {
        return Err(StdError::generic_err("auction is below minimum bid"));
    }

    if current_bid.amount <= auction.bid_amount {
        return Err(StdError::generic_err("must bid above current top bid"));
    }

    auction.top_bidder = Some(info.sender.clone().into_string());
    auction.bid_amount = current_bid.amount;

    let mut messages = vec![];

    if let AssetInfo::Token { contract_addr } = asset.info.clone() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg: to_binary(&TokenMsg::TransferFrom {
                owner: info.sender.clone().into_string(),
                recipient: env.contract.address.clone().into_string(),
                amount: asset.amount,
            })?,
            funds: vec![],
        }))
    }

    AUCTION_BIDS.save(
        deps.storage,
        (&auction_id.to_be_bytes(), info.sender.as_bytes()),
        &current_bid,
    )?;
    AUCTIONS.save(deps.storage, &auction_id.to_be_bytes(), &auction)?;

    Ok(Response::new().add_messages(messages))
}
