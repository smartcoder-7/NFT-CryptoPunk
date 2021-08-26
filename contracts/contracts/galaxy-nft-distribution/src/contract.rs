use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{
    new_id, Config, Reservation, CONFIG, RESERVATIONS, RESERVATION_COUNT, SALE_COUNT,
    WITHDRAW_COUNT,
};
use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Response,
    StdError, StdResult, Uint128, WasmMsg,
};
use cw721_base::msg::{ExecuteMsg as Cw721ExecuteMsg, MintMsg};
use terraswap::asset::Asset;

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let cfg = Config {
        owner: msg.owner,
        cost: msg.cost.clone(),
        nft_contract: None,
        limit_per_address: msg.limit_per_address,
        nft_limit: msg.nft_limit,
        response_seconds: msg.response_seconds,
    };

    if !msg.cost.is_native_token() {
        return Err(StdError::generic_err("cost must be native token"));
    }

    SALE_COUNT.save(deps.storage, &0)?;
    WITHDRAW_COUNT.save(deps.storage, &0)?;
    CONFIG.save(deps.storage, &cfg)?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(deps: DepsMut, env: Env, info: MessageInfo, msg: ExecuteMsg) -> StdResult<Response> {
    match msg {
        ExecuteMsg::SetNftContract { contract_addr } => set_nft_contract(deps, info, contract_addr),
        ExecuteMsg::ReserveNft {} => reserve_nft(deps, env, info),
        ExecuteMsg::RefundNft { reservation_id } => refund_nft(deps, env, info, reservation_id),
        ExecuteMsg::MintNft {
            reservation_id,
            token_id,
            name,
            description,
            image,
        } => mint_nft(
            deps,
            info,
            reservation_id,
            token_id,
            name,
            description,
            image,
        ),
        ExecuteMsg::WithdrawSales {} => withdraw_sales(deps),
    }
}

pub fn withdraw_sales(deps: DepsMut) -> StdResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;

    let sale_count = SALE_COUNT.load(deps.storage)?;
    let withdraw_count = WITHDRAW_COUNT.load(deps.storage)?;

    let to_withdraw = Asset {
        info: cfg.cost.info,
        amount: Uint128::from(cfg.cost.amount.u128() * ((sale_count - withdraw_count) as u128)),
    };
    WITHDRAW_COUNT.save(deps.storage, &sale_count)?;
    Ok(Response::new()
        .add_message(to_withdraw.into_msg(&deps.querier, Addr::unchecked(cfg.owner))?))
}

pub fn mint_nft(
    deps: DepsMut,
    info: MessageInfo,
    reservation_id: u64,
    token_id: String,
    name: String,
    description: Option<String>,
    image: Option<String>,
) -> StdResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;
    if info.sender.clone().into_string() != cfg.owner {
        return Err(StdError::generic_err("not authorized"));
    }

    let sale_count = SALE_COUNT.load(deps.storage)?;

    if sale_count == cfg.nft_limit {
        return Err(StdError::generic_err("cannot mint more than nft limit"));
    }

    let mut reservation = RESERVATIONS.load(deps.storage, &reservation_id.to_be_bytes())?;

    if !reservation.valid {
        return Err(StdError::generic_err("reservation is invalid"));
    }

    reservation.valid = false;
    RESERVATIONS.save(deps.storage, &reservation_id.to_be_bytes(), &reservation)?;

    SALE_COUNT.save(deps.storage, &(sale_count + 1))?;

    Ok(
        Response::new().add_message(CosmosMsg::Wasm(WasmMsg::Execute {
            contract_addr: cfg.nft_contract.unwrap(),
            msg: to_binary(&Cw721ExecuteMsg::Mint {
                0: MintMsg {
                    token_id,
                    owner: reservation.owner,
                    name,
                    description,
                    image,
                },
            })?,
            funds: vec![],
        })),
    )
}

pub fn refund_nft(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    reservation_id: u64,
) -> StdResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;

    let mut reservation = RESERVATIONS.load(deps.storage, &reservation_id.to_be_bytes())?;
    if reservation.owner != info.sender.into_string() {
        return Err(StdError::generic_err("reservation is not owned by sender"));
    }

    if !reservation.valid {
        return Err(StdError::generic_err("reservation is invalid"));
    }

    if env.block.time.seconds() < reservation.refundable_at {
        return Err(StdError::generic_err("reservation cannot be refunded yet"));
    }

    reservation.valid = false;
    RESERVATIONS.save(deps.storage, &reservation_id.to_be_bytes(), &reservation)?;
    Ok(Response::new().add_message(
        cfg.cost
            .into_msg(&deps.querier, Addr::unchecked(reservation.owner))?,
    ))
}

pub fn reserve_nft(deps: DepsMut, env: Env, info: MessageInfo) -> StdResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;
    cfg.cost.assert_sent_native_token_balance(&info)?;

    let count = RESERVATION_COUNT.load(deps.storage, info.sender.as_bytes())?;
    if count == cfg.limit_per_address {
        return Err(StdError::generic_err("this address cannot mint more nfts"));
    }
    RESERVATION_COUNT.save(deps.storage, info.sender.as_bytes(), &(count + 1))?;

    let reservation_n = new_id(deps.storage)?;
    let reservation = Reservation {
        owner: info.sender.clone().into_string(),
        valid: true,
        refundable_at: env.block.time.seconds() + cfg.response_seconds,
    };

    RESERVATIONS.save(deps.storage, &reservation_n.to_be_bytes(), &reservation)?;

    if info.sender.into_string() != cfg.owner {
        return Err(StdError::generic_err("not authorized"));
    }
    Ok(Response::new())
}

pub fn set_nft_contract(
    deps: DepsMut,
    info: MessageInfo,
    contract_addr: String,
) -> StdResult<Response> {
    let mut cfg = CONFIG.load(deps.storage)?;
    if cfg.nft_contract.is_some() || cfg.owner != info.sender.into_string() {
        return Err(StdError::generic_err("not authorized"));
    }

    cfg.nft_contract = Some(contract_addr);
    CONFIG.save(deps.storage, &cfg)?;
    Ok(Response::new())
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(_deps: Deps, _env: Env, _msg: QueryMsg) -> StdResult<Binary> {
    Err(StdError::generic_err("no queries"))
}
