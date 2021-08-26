use crate::state::{new_id, SellOrder, CONFIG, FEE_STORAGE, SELL_ORDERS};
use cosmwasm_std::{
    to_binary, Addr, CosmosMsg, DepsMut, Env, MessageInfo, Response, StdError, StdResult, WasmMsg,
};
use cw20_base::msg::ExecuteMsg as TokenMsg;
use cw721::Cw721ExecuteMsg;
use terraswap::asset::{Asset, AssetInfo};

pub fn create_sell_order(
    deps: DepsMut,
    info: MessageInfo,
    creator: String,
    token_id: String,
    requested_asset: Asset,
) -> StdResult<Response> {
    let sell_order = SellOrder {
        nft_contract: info.sender.into_string(),
        token_id,
        creator,
        requested_asset,
        cancelled: false,
    };

    let order_id = new_id(deps.storage)?;
    SELL_ORDERS.save(deps.storage, &order_id.to_be_bytes(), &sell_order)?;
    Ok(Response::new())
}

pub fn fill_sell_order(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    order_id: u64,
) -> StdResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;
    let mut order = SELL_ORDERS.load(deps.storage, &order_id.to_be_bytes())?;

    if order.cancelled {
        return Err(StdError::generic_err("order has been cancelled or filled"));
    }

    order.cancelled = true;
    SELL_ORDERS.save(deps.storage, &order_id.to_be_bytes(), &order)?;

    order
        .requested_asset
        .assert_sent_native_token_balance(&info)?;

    let mut messages = vec![];

    if let AssetInfo::Token { contract_addr } = order.requested_asset.info.clone() {
        messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr,
            msg: to_binary(&TokenMsg::TransferFrom {
                owner: info.sender.clone().into_string(),
                recipient: env.contract.address.clone().into_string(),
                amount: order.requested_asset.amount,
            })?,
            funds: vec![],
        }))
    }

    let fee_amt = order.requested_asset.amount * cfg.fee;
    let to_creator = Asset {
        info: order.requested_asset.info.clone(),
        amount: order.requested_asset.amount - fee_amt,
    };

    let mut current_fee = FEE_STORAGE.load(
        deps.storage,
        order.requested_asset.info.to_string().as_bytes(),
    )?;
    current_fee += fee_amt;

    FEE_STORAGE.save(
        deps.storage,
        order.requested_asset.info.to_string().as_bytes(),
        &current_fee,
    )?;

    messages.push(to_creator.into_msg(&deps.querier, Addr::unchecked(order.creator))?);
    messages.push(CosmosMsg::Wasm(WasmMsg::Execute {
        contract_addr: order.nft_contract.clone(),
        msg: to_binary(&Cw721ExecuteMsg::TransferNft {
            recipient: info.sender.into_string(),
            token_id: order.token_id,
        })?,
        funds: vec![],
    }));
    Ok(Response::new().add_messages(messages))
}

pub fn cancel_sell_order(deps: DepsMut, info: MessageInfo, order_id: u64) -> StdResult<Response> {
    let mut order = SELL_ORDERS.load(deps.storage, &order_id.to_be_bytes())?;
    if info.sender.clone().into_string() != order.creator {
        return Err(StdError::generic_err("unauthorized"));
    }

    order.cancelled = true;
    SELL_ORDERS.save(deps.storage, &order_id.to_be_bytes(), &order)?;

    Ok(
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: order.nft_contract.clone(),
            msg: to_binary(&Cw721ExecuteMsg::TransferNft {
                recipient: info.sender.into_string(),
                token_id: order.token_id,
            })?,
            funds: vec![],
        })),
    )
}
