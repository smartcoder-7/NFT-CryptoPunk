use crate::auction::{close_auction, create_auction, increase_auction_bid, withdraw_auction_bid};
use crate::order::{cancel_sell_order, create_sell_order, fill_sell_order};
use crate::state::{Config, CONFIG, FEE_STORAGE};
use cosmwasm_std::{
    entry_point, from_binary, Addr, Binary, Decimal, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Uint128,
};
use cw721::Cw721ReceiveMsg;
use stardust_protocol::nft_auction::{Cw721HookMsg, ExecuteMsg, InstantiateMsg, QueryMsg};
use terraswap::asset::{Asset, AssetInfo};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let cfg = Config {
        owner: msg.owner,
        fee: msg.fee,
    };

    CONFIG.save(deps.storage, &cfg)?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::ReceiveNft(receive_msg) => receive_nft(deps, info, receive_msg),
        ExecuteMsg::UpdateConfig { owner, fee } => update_config(deps, info, owner, fee),
        ExecuteMsg::IncreaseAuctionBid { auction_id, asset } => {
            increase_auction_bid(deps, env, info, auction_id, asset)
        }
        ExecuteMsg::WithdrawAuctionBid { auction_id } => {
            withdraw_auction_bid(deps, info, auction_id)
        }
        ExecuteMsg::CloseAuction {
            auction_id,
            require_no_bidder,
        } => close_auction(deps, env, info, auction_id, require_no_bidder),
        ExecuteMsg::FillSellOrder { order_id } => fill_sell_order(deps, env, info, order_id),
        ExecuteMsg::CancelSellOrder { order_id } => cancel_sell_order(deps, info, order_id),
        ExecuteMsg::WithdrawFees {
            asset_info,
            address,
        } => withdraw_fees(deps, info, asset_info, address),
    }
}

pub fn update_config(
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
    fee: Decimal,
) -> StdResult<Response> {
    let mut cfg = CONFIG.load(deps.storage)?;
    if info.sender.into_string() != cfg.owner {
        return Err(StdError::generic_err("unauthorized"));
    }
    cfg.owner = owner;
    cfg.fee = fee;
    CONFIG.save(deps.storage, &cfg)?;

    Ok(Response::new())
}

pub fn withdraw_fees(
    deps: DepsMut,
    info: MessageInfo,
    asset_info: AssetInfo,
    address: String,
) -> StdResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;
    if info.sender.into_string() != cfg.owner {
        return Err(StdError::generic_err("unauthorized"));
    }

    let fee_amount = FEE_STORAGE.load(deps.storage, asset_info.to_string().as_bytes())?;
    FEE_STORAGE.save(
        deps.storage,
        asset_info.to_string().as_bytes(),
        &Uint128::zero(),
    )?;

    let asset = Asset {
        info: asset_info,
        amount: fee_amount,
    };

    Ok(Response::new().add_message(asset.into_msg(&deps.querier, Addr::unchecked(address))?))
}

pub fn receive_nft(
    deps: DepsMut,
    info: MessageInfo,
    receive_msg: Cw721ReceiveMsg,
) -> StdResult<Response> {
    match from_binary(&receive_msg.msg).unwrap() {
        Cw721HookMsg::CreateAuction {
            min_bid,
            expiration_date,
            denom,
        } => create_auction(
            deps,
            info,
            receive_msg.sender,
            receive_msg.token_id,
            min_bid,
            expiration_date,
            denom,
        ),
        Cw721HookMsg::CreateSellOrder { requested_asset } => create_sell_order(
            deps,
            info,
            receive_msg.sender,
            receive_msg.token_id,
            requested_asset,
        ),
    }
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    Err(StdError::generic_err("no queries"))
}
