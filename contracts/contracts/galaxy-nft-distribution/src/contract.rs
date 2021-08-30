use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};
use crate::state::{
    invalidate_reservation, new_id, Config, DistributionStatus, Reservation, CONFIG,
    DISTRIBUTION_STATUS, RESERVATIONS, RESERVATIONS_BY_ADDRESS, UNIQUE_ID, VALID_RESERVATIONS,
};
use cosmwasm_std::{
    entry_point, to_binary, Addr, Binary, CosmosMsg, Deps, DepsMut, Env, MessageInfo, Order,
    Response, StdError, StdResult, Uint128, WasmMsg,
};
use cw721_base::msg::{ExecuteMsg as Cw721ExecuteMsg, MintMsg};
use cw_storage_plus::Bound;
use std::cmp::min;
use std::convert::TryInto;
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

    UNIQUE_ID.save(deps.storage, &0)?;
    CONFIG.save(deps.storage, &cfg)?;

    let distribution_status = DistributionStatus {
        withdraw_count: 0,
        sale_count: 0,
        valid_reservations_count: 0,
        total_reservation_count: 0,
        nft_limit: msg.nft_limit,
    };

    DISTRIBUTION_STATUS.save(deps.storage, &distribution_status)?;
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
    let mut distribution_status = DISTRIBUTION_STATUS.load(deps.storage)?;

    let to_withdraw = Asset {
        info: cfg.cost.info,
        amount: Uint128::from(
            cfg.cost.amount.u128()
                * ((distribution_status.sale_count - distribution_status.withdraw_count) as u128),
        ),
    };
    distribution_status.withdraw_count = distribution_status.sale_count;

    DISTRIBUTION_STATUS.save(deps.storage, &distribution_status)?;
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

    let mut distribution_status = DISTRIBUTION_STATUS.load(deps.storage)?;

    if distribution_status.sale_count == cfg.nft_limit {
        return Err(StdError::generic_err("cannot mint more than nft limit"));
    }
    distribution_status.sale_count += 1;
    DISTRIBUTION_STATUS.save(deps.storage, &distribution_status)?;

    let reservation = RESERVATIONS.load(deps.storage, &reservation_id.to_be_bytes())?;

    invalidate_reservation(deps.storage, reservation_id)?;

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

    let reservation = RESERVATIONS.load(deps.storage, &reservation_id.to_be_bytes())?;
    if reservation.owner != info.sender.into_string() {
        return Err(StdError::generic_err("reservation is not owned by sender"));
    }

    if env.block.time.seconds() < reservation.refundable_at {
        return Err(StdError::generic_err("reservation cannot be refunded yet"));
    }

    invalidate_reservation(deps.storage, reservation_id)?;

    Ok(Response::new().add_message(
        cfg.cost
            .into_msg(&deps.querier, Addr::unchecked(reservation.owner))?,
    ))
}

pub fn reserve_nft(deps: DepsMut, env: Env, info: MessageInfo) -> StdResult<Response> {
    let cfg = CONFIG.load(deps.storage)?;
    cfg.cost.assert_sent_native_token_balance(&info)?;

    let mut by_address = RESERVATIONS_BY_ADDRESS
        .load(deps.storage, info.sender.as_bytes())
        .unwrap_or(vec![]);

    if by_address.len() == cfg.limit_per_address as usize {
        return Err(StdError::generic_err("this address cannot mint more nfts"));
    }

    let mut distribution_status = DISTRIBUTION_STATUS.load(deps.storage)?;
    distribution_status.valid_reservations_count += 1;
    distribution_status.total_reservation_count += 1;
    DISTRIBUTION_STATUS.save(deps.storage, &distribution_status)?;

    let reservation_n = new_id(deps.storage)?;
    let reservation = Reservation {
        owner: info.sender.clone().into_string(),
        valid: true,
        refundable_at: env.block.time.seconds() + cfg.response_seconds,
    };

    by_address.push(reservation_n);
    RESERVATIONS_BY_ADDRESS.save(deps.storage, info.sender.as_bytes(), &by_address)?;
    RESERVATIONS.save(deps.storage, &reservation_n.to_be_bytes(), &reservation)?;
    VALID_RESERVATIONS.save(deps.storage, &reservation_n.to_be_bytes(), &true)?;

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
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    /*
    TODO: Queries:
    - Current state
    - Reservation by ID
    - All reservations (paginate?)
     */

    match msg {
        QueryMsg::ReservationById { id } => to_binary(&reservation_by_id(deps, id)?),
        QueryMsg::ReservationsByAddress { address } => {
            to_binary(&reservations_by_address(deps, address)?)
        }
        QueryMsg::ValidReservations { start_at, limit } => {
            to_binary(&valid_reservations(deps, start_at, limit)?)
        }
        QueryMsg::DistributionStatus {} => to_binary(&distribution_status(deps)?),
    }
}

pub fn reservation_by_id(deps: Deps, id: u64) -> StdResult<Reservation> {
    let reservation = RESERVATIONS.load(deps.storage, &id.to_be_bytes())?;
    Ok(reservation)
}

pub fn reservations_by_address(deps: Deps, address: String) -> StdResult<Vec<Reservation>> {
    let reservation_ids = RESERVATIONS_BY_ADDRESS.load(deps.storage, address.as_bytes())?;
    let mut reservations = vec![];
    for id in reservation_ids {
        let res = RESERVATIONS.load(deps.storage, &id.to_be_bytes())?;
        reservations.push(res)
    }
    Ok(reservations)
}

pub fn valid_reservations(deps: Deps, start_at: u64, mut limit: u64) -> StdResult<Vec<u64>> {
    limit = min(limit, 32u64);
    VALID_RESERVATIONS
        .range(
            deps.storage,
            Some(Bound::inclusive(start_at.to_be_bytes())),
            None,
            Order::Ascending,
        )
        .take(limit as usize)
        .map(|item| {
            let (id, _) = item?;
            Ok(u64::from_be_bytes(id.as_slice().try_into().unwrap()))
        })
        .collect()
}

pub fn distribution_status(deps: Deps) -> StdResult<DistributionStatus> {
    return DISTRIBUTION_STATUS.load(deps.storage);
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::{coins, SubMsg};
    use terraswap::asset::AssetInfo;

    #[test]
    fn happy_path() {
        let mut deps = mock_dependencies(&[]);
        let env = mock_env();

        let creator_info = mock_info("creator", &coins(1000, "uluna"));

        // Create the distribution contract
        let create_msg = InstantiateMsg {
            owner: creator_info.sender.to_string(),
            cost: Asset {
                info: AssetInfo::NativeToken {
                    denom: "uluna".to_string(),
                },
                amount: Uint128::new(100),
            },
            nft_contract: None,
            limit_per_address: 5,
            nft_limit: 10,
            response_seconds: 10000,
        };
        let create_res =
            instantiate(deps.as_mut(), env.clone(), creator_info.clone(), create_msg).unwrap();
        assert_eq!(0, create_res.messages.len());

        // Set smart contract address
        let set_contract_msg = ExecuteMsg::SetNftContract {
            contract_addr: String::from("contract_addr"),
        };
        execute(
            deps.as_mut(),
            env.clone(),
            creator_info.clone(),
            set_contract_msg,
        )
        .unwrap();

        // Try to create a reservation
        let purchaser_info = mock_info("purchaser", &coins(100, "uluna"));
        let create_reservation_msg = ExecuteMsg::ReserveNft {};
        execute(
            deps.as_mut(),
            env.clone(),
            purchaser_info.clone(),
            create_reservation_msg,
        )
        .unwrap();

        // Mint NFT
        let token_id = "0".to_string();
        let token_name = "token".to_string();
        let mint_nft_msg = ExecuteMsg::MintNft {
            reservation_id: 0,
            token_id: token_id.clone(),
            name: token_name.clone(),
            description: None,
            image: None,
        };
        let mint_result = execute(
            deps.as_mut(),
            env.clone(),
            creator_info.clone(),
            mint_nft_msg,
        )
        .unwrap();
        assert_eq!(
            mint_result.messages[0],
            SubMsg::new(CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "contract_addr".to_string(),
                msg: to_binary(&Cw721ExecuteMsg::Mint {
                    0: MintMsg {
                        token_id: token_id.clone(),
                        owner: purchaser_info.sender.clone().to_string(),
                        name: token_name.clone(),
                        description: None,
                        image: None,
                    },
                })
                .unwrap(),
                funds: vec![],
            }))
        );
    }

    /*
    TODO: Test cases:
    - Reserve with insufficient cost
    - Reserve with # > limit_per_address
    - Reserve with # > max_nft
    - Refund happy path
    - Refund with already refunded reservation
    - Withdraw sales happy path
    - Withdraw sales unauthorized
     */
}
