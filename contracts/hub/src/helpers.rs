use std::str::FromStr;

use cosmwasm_std::{
    Addr, BalanceResponse, BankQuery, Coin, CosmosMsg, QuerierWrapper, QueryRequest, Reply,
    StdError, StdResult, SubMsgResponse, Uint128,
};
use cw20::{Cw20QueryMsg, TokenInfoResponse};

use crate::types::Delegation;

/// Unwrap a `Reply` object to extract the response
pub(crate) fn unwrap_reply(reply: Reply) -> StdResult<SubMsgResponse> {
    reply.result.into_result().map_err(StdError::generic_err)
}

/// Query the total supply of a CW20 token
pub(crate) fn query_cw20_total_supply(
    querier: &QuerierWrapper,
    token_addr: &Addr,
) -> StdResult<Uint128> {
    let token_info: TokenInfoResponse =
        querier.query_wasm_smart(token_addr, &Cw20QueryMsg::TokenInfo {})?;
    Ok(token_info.total_supply)
}

/// Query the amounts of Native Token a staker is delegating to a specific validator
pub(crate) fn query_delegation(
    querier: &QuerierWrapper,
    validator: &str,
    delegator_addr: &Addr,
    denom: &str,
) -> StdResult<Delegation> {
    Ok(Delegation {
        validator: validator.to_string(),
        amount: querier
            .query_delegation(delegator_addr, validator)?
            .map(|fd| fd.amount.amount.u128())
            .unwrap_or(0),
        denom: denom.into(),
    })
}

/// Query the amounts of Native Token a staker is delegating to each of the validators specified
pub(crate) fn query_delegations(
    querier: &QuerierWrapper,
    validators: &[String],
    delegator_addr: &Addr,
    denom: &str,
) -> StdResult<Vec<Delegation>> {
    validators
        .iter()
        .map(|validator| query_delegation(querier, validator, delegator_addr, denom))
        .collect()
}

/// `cosmwasm_std::Coin` does not implement `FromStr`, so we have do it ourselves
///
/// Parsing the string with regex doesn't work, because the resulting binary would be too big for
/// including the `regex` library. Example:
/// https://github.com/PFC-Validator/terra-rust/blob/v1.1.8/terra-rust-api/src/client/core_types.rs#L34-L55
///
/// We opt for a dirtier solution. Enumerate characters in the string, and break before the first
/// character that is not a number. Split the string at that index.
///
/// This assumes the denom never starts with a number, which is true on Terra.
pub(crate) fn parse_coin(s: &str) -> StdResult<Coin> {
    for (i, c) in s.chars().enumerate() {
        if c.is_alphabetic() {
            let amount = Uint128::from_str(&s[..i])?;
            let denom = &s[i..];
            return Ok(Coin::new(amount.u128(), denom));
        }
    }

    Err(StdError::generic_err(format!(
        "failed to parse coin: {}",
        s
    )))
}

/// Find the amount of a denom sent along a message, assert it is non-zero, and no other denom were
/// sent together
pub(crate) fn parse_received_fund(funds: &[Coin], denom: &str) -> StdResult<Uint128> {
    if funds.len() != 1 {
        return Err(StdError::generic_err(format!(
            "must deposit exactly one coin; received {}",
            funds.len()
        )));
    }

    let fund = &funds[0];
    if fund.denom != denom {
        return Err(StdError::generic_err(format!(
            "expected {} deposit, received {}",
            denom, fund.denom
        )));
    }

    if fund.amount.is_zero() {
        return Err(StdError::generic_err("deposit amount must be non-zero"));
    }

    Ok(fund.amount)
}

pub fn get_denom_balance(
    querier: &QuerierWrapper,
    account_addr: Addr,
    denom: String,
) -> StdResult<Uint128> {
    let balance: BalanceResponse = querier.query(&QueryRequest::Bank(BankQuery::Balance {
        address: account_addr.to_string(),
        denom,
    }))?;
    Ok(balance.amount.amount)
}

// encode a protobuf into a cosmos message
// Inspired by https://github.com/alice-ltd/smart-contracts/blob/master/contracts/alice_terra_token/src/execute.rs#L73-L76
pub(crate) fn proto_encode<M: prost::Message>(msg: M, type_url: String) -> StdResult<CosmosMsg> {
    let mut bytes = Vec::new();
    prost::Message::encode(&msg, &mut bytes)
        .map_err(|e| StdError::generic_err("Message encoding must be infallible"))?;
    Ok(cosmwasm_std::CosmosMsg::<cosmwasm_std::Empty>::Stargate {
        type_url,
        value: cosmwasm_std::Binary(bytes),
    })
    // let mut msg_buf: Vec<u8> = vec![];
    // msg.encode(&mut msg_buf)
    //     .map_err(|_e| ContractError::Std(StdError::generic_err("failed to compile proto")))?;
    // Ok(msg_buf)
}
