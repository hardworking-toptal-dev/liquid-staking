use std::str::FromStr;

use cosmwasm_std::testing::{mock_env, mock_info, MockApi, MockStorage, MOCK_CONTRACT_ADDR};
use cosmwasm_std::{
    to_binary, Addr, Coin, CosmosMsg, Decimal, Event, OwnedDeps, Reply, ReplyOn, StakingMsg,
    StdError, SubMsg, SubMsgExecutionResponse, Uint128, WasmMsg,
};
use cw20::{Cw20ExecuteMsg, MinterResponse};
use cw20_base::msg::InstantiateMsg as Cw20InstantiateMsg;

use steak::hub::{
    Batch, ConfigResponse, ExecuteMsg, InstantiateMsg, PendingBatch, QueryMsg, StateResponse,
    UnbondRequest, UnbondRequestsByBatchResponseItem, UnbondRequestsByUserResponseItem,
};

use crate::contract::{execute, instantiate, reply};
use crate::helpers::{parse_coin, parse_received_fund};
use crate::math::{compute_delegations, compute_undelegations};
use crate::state::State;
use crate::types::{Coins, Delegation, Undelegation};

use super::custom_querier::CustomQuerier;
use super::helpers::{mock_dependencies, mock_env_with_timestamp, query_helper};

//--------------------------------------------------------------------------------------------------
// Test setup
//--------------------------------------------------------------------------------------------------

fn setup_test() -> OwnedDeps<MockStorage, MockApi, CustomQuerier> {
    let mut deps = mock_dependencies();

    let res = instantiate(
        deps.as_mut(),
        mock_env_with_timestamp(10000),
        mock_info("deployer", &[]),
        InstantiateMsg {
            cw20_code_id: 69420,
            admin: "admin".to_string(),
            name: "Steak Token".to_string(),
            symbol: "STEAK".to_string(),
            decimals: 6,
            epoch_period: 259200,   // 3 * 24 * 60 * 60 = 3 days
            unbond_period: 1814400, // 21 * 24 * 60 * 60 = 21 days
            validators: vec!["alice".to_string(), "bob".to_string(), "charlie".to_string()],
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 1);
    assert_eq!(
        res.messages[0],
        SubMsg::reply_on_success(
            CosmosMsg::Wasm(WasmMsg::Instantiate {
                admin: Some("admin".to_string()),
                code_id: 69420,
                msg: to_binary(&Cw20InstantiateMsg {
                    name: "Steak Token".to_string(),
                    symbol: "STEAK".to_string(),
                    decimals: 6,
                    initial_balances: vec![],
                    mint: Some(MinterResponse {
                        minter: MOCK_CONTRACT_ADDR.to_string(),
                        cap: None
                    }),
                    marketing: None,
                })
                .unwrap(),
                funds: vec![],
                label: "steak_token".to_string()
            }),
            1
        )
    );

    let event = Event::new("instantiate_contract")
        .add_attribute("creator", MOCK_CONTRACT_ADDR)
        .add_attribute("admin", "admin")
        .add_attribute("code_id", "69420")
        .add_attribute("contract_address", "steak_token");

    let res = reply(
        deps.as_mut(),
        mock_env_with_timestamp(10000),
        Reply {
            id: 1,
            result: cosmwasm_std::ContractResult::Ok(SubMsgExecutionResponse {
                events: vec![event],
                data: None,
            }),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 0);

    deps.querier.set_cw20_total_supply("steak_token", 0);
    deps
}

//--------------------------------------------------------------------------------------------------
// Execution
//--------------------------------------------------------------------------------------------------

#[test]
fn proper_instantiation() {
    let deps = setup_test();

    let res: ConfigResponse = query_helper(deps.as_ref(), QueryMsg::Config {});
    assert_eq!(
        res,
        ConfigResponse {
            steak_token: "steak_token".to_string(),
            epoch_period: 259200,
            unbond_period: 1814400,
            validators: vec!["alice".to_string(), "bob".to_string(), "charlie".to_string()]
        }
    );

    let res: StateResponse = query_helper(deps.as_ref(), QueryMsg::State {});
    assert_eq!(
        res,
        StateResponse {
            total_usteak: Uint128::zero(),
            total_uluna: Uint128::zero(),
            exchange_rate: Decimal::one(),
            unlocked_coins: vec![],
        }
    );

    let res: PendingBatch = query_helper(deps.as_ref(), QueryMsg::PendingBatch {});
    assert_eq!(
        res,
        PendingBatch {
            id: 1,
            usteak_to_burn: Uint128::zero(),
            est_unbond_start_time: 269200, // 10000 + 259200
        }
    );
}

#[test]
fn bonding() {
    let mut deps = setup_test();

    // Bond when no delegation has been made
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("user_1", &[Coin::new(1000000, "uluna")]),
        ExecuteMsg::Bond {
            receiver: None,
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 4);
    assert_eq!(
        res.messages[0],
        SubMsg::reply_on_success(Delegation::new("alice", 333334u128).to_cosmos_msg(), 2)
    );
    assert_eq!(
        res.messages[1],
        SubMsg::reply_on_success(Delegation::new("bob", 333333u128).to_cosmos_msg(), 2)
    );
    assert_eq!(
        res.messages[2],
        SubMsg::reply_on_success(Delegation::new("charlie", 333333u128).to_cosmos_msg(), 2)
    );
    assert_eq!(
        res.messages[3],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "steak_token".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: "user_1".to_string(),
                    amount: Uint128::new(1000000)
                })
                .unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );

    // Bond when there are existing delegations, and Luna:Steak exchange rate is >1
    // Previously user 1 delegated 1000000 uluna. We assume we have accumulated 2.5% yield at 1025000 staked
    deps.querier.set_staking_delegations(&[
        Delegation::new("alice", 341667u128),
        Delegation::new("bob", 341667u128),
        Delegation::new("charlie", 341666u128),
    ]);
    deps.querier.set_cw20_total_supply("steak_token", 1000000);

    // Target = (1025000 + 12345) / 3 = 345781
    // Remainder = 2
    // Alice:   345781 + 1 - 341667 = 4115
    // Bob:     345781 + 1 - 341667 = 4115
    // Charlie: 345781 + 0 - 341666 = 4115
    // Mint amount: 12345 * 1000000 / 1025000 = 12043
    let res = execute(
        deps.as_mut(),
        mock_env(),
        mock_info("user_2", &[Coin::new(12345, "uluna")]),
        ExecuteMsg::Bond {
            receiver: Some("user_3".to_string()),
        },
    )
    .unwrap();

    assert_eq!(res.messages.len(), 4);
    assert_eq!(
        res.messages[0],
        SubMsg::reply_on_success(Delegation::new("alice", 4115u128).to_cosmos_msg(), 2)
    );
    assert_eq!(
        res.messages[1],
        SubMsg::reply_on_success(Delegation::new("bob", 4115u128).to_cosmos_msg(), 2)
    );
    assert_eq!(
        res.messages[2],
        SubMsg::reply_on_success(Delegation::new("charlie", 4115u128).to_cosmos_msg(), 2)
    );
    assert_eq!(
        res.messages[3],
        SubMsg {
            id: 0,
            msg: CosmosMsg::Wasm(WasmMsg::Execute {
                contract_addr: "steak_token".to_string(),
                msg: to_binary(&Cw20ExecuteMsg::Mint {
                    recipient: "user_3".to_string(),
                    amount: Uint128::new(12043)
                })
                .unwrap(),
                funds: vec![]
            }),
            gas_limit: None,
            reply_on: ReplyOn::Never
        }
    );

    // Check the state after bonding
    deps.querier.set_staking_delegations(&[
        Delegation::new("alice", 345782u128),
        Delegation::new("bob", 345782u128),
        Delegation::new("charlie", 345781u128),
    ]);
    deps.querier.set_cw20_total_supply("steak_token", 1012043);

    let res: StateResponse = query_helper(deps.as_ref(), QueryMsg::State {});
    assert_eq!(
        res,
        StateResponse {
            total_usteak: Uint128::new(1012043),
            total_uluna: Uint128::new(1037345),
            exchange_rate: Decimal::from_ratio(1037345u128, 1012043u128),
            unlocked_coins: vec![],
        }
    );
}

// #[test]
// fn harvesting() {
//     let mut deps = setup_test();
// }

// #[test]
// fn queuing_unbond() {
//     let mut deps = setup_test();
// }

// #[test]
// fn submitting_batch() {
//     let mut deps = setup_test();
// }

// #[test]
// fn withdrawing_unbonded() {
//     let mut deps = setup_test();
// }

//--------------------------------------------------------------------------------------------------
// Queries
//--------------------------------------------------------------------------------------------------

#[test]
fn querying_previous_batches() {
    let mut deps = mock_dependencies();

    let batches = vec![
        Batch {
            id: 1,
            total_shares: Uint128::new(123),
            uluna_unclaimed: Uint128::new(456),
            est_unbond_end_time: 10000,
        },
        Batch {
            id: 2,
            total_shares: Uint128::new(345),
            uluna_unclaimed: Uint128::new(456),
            est_unbond_end_time: 15000,
        },
    ];

    let state = State::default();
    for batch in &batches {
        state.previous_batches.save(deps.as_mut().storage, batch.id.into(), batch).unwrap();
    }

    let res: Vec<Batch> = query_helper(
        deps.as_ref(),
        QueryMsg::PreviousBatches {
            start_after: None,
            limit: None,
        },
    );
    assert_eq!(res, batches.clone());

    let res: Vec<Batch> = query_helper(
        deps.as_ref(),
        QueryMsg::PreviousBatches {
            start_after: Some(1),
            limit: None,
        },
    );
    assert_eq!(res, vec![batches[1].clone()]);

    let res: Vec<Batch> = query_helper(
        deps.as_ref(),
        QueryMsg::PreviousBatches {
            start_after: Some(2),
            limit: None,
        },
    );
    assert_eq!(res, vec![]);
}

#[test]
fn querying_unbond_shares() {
    let mut deps = mock_dependencies();

    let unbond_shares = vec![
        UnbondRequest {
            id: 1,
            user: String::from("alice"),
            shares: Uint128::new(123),
        },
        UnbondRequest {
            id: 1,
            user: String::from("bob"),
            shares: Uint128::new(234),
        },
        UnbondRequest {
            id: 1,
            user: String::from("charlie"),
            shares: Uint128::new(345),
        },
        UnbondRequest {
            id: 2,
            user: String::from("alice"),
            shares: Uint128::new(456),
        },
    ];

    let state = State::default();
    for unbond_share in &unbond_shares {
        state
            .unbond_requests
            .save(
                deps.as_mut().storage,
                (unbond_share.id.into(), &Addr::unchecked(unbond_share.user.clone())),
                unbond_share,
            )
            .unwrap();
    }

    let res: Vec<UnbondRequestsByBatchResponseItem> = query_helper(
        deps.as_ref(),
        QueryMsg::UnbondRequestsByBatch {
            id: 1,
            start_after: None,
            limit: None,
        },
    );
    assert_eq!(
        res,
        vec![
            unbond_shares[0].clone().into(),
            unbond_shares[1].clone().into(),
            unbond_shares[2].clone().into()
        ]
    );

    let res: Vec<UnbondRequestsByBatchResponseItem> = query_helper(
        deps.as_ref(),
        QueryMsg::UnbondRequestsByBatch {
            id: 2,
            start_after: None,
            limit: None,
        },
    );
    assert_eq!(res, vec![unbond_shares[3].clone().into()]);

    let res: Vec<UnbondRequestsByUserResponseItem> = query_helper(
        deps.as_ref(),
        QueryMsg::UnbondRequestsByUser {
            user: "alice".to_string(),
            start_after: None,
            limit: None,
        },
    );
    assert_eq!(res, vec![unbond_shares[0].clone().into(), unbond_shares[3].clone().into()]);
}

//--------------------------------------------------------------------------------------------------
// Delegations/undelegations
//--------------------------------------------------------------------------------------------------

#[test]
fn computing_delegations() {
    // Scenario 1: The contract is freshly instantiated, and has not made any delegation yet
    let current_delegations = vec![
        Delegation::new("alice", 0u128),
        Delegation::new("bob", 0u128),
        Delegation::new("charlie", 0u128),
    ];

    // If the amount can be evenly distributed across validators...
    let new_delegations = compute_delegations(Uint128::new(333), &current_delegations);
    let expected = vec![
        Delegation::new("alice", 111u128),
        Delegation::new("bob", 111u128),
        Delegation::new("charlie", 111u128),
    ];
    assert_eq!(new_delegations, expected);

    // If the amount can NOT be evenly distributed across validators...
    let new_delegations = compute_delegations(Uint128::new(334), &current_delegations);
    let expected = vec![
        Delegation::new("alice", 112u128),
        Delegation::new("bob", 111u128),
        Delegation::new("charlie", 111u128),
    ];
    assert_eq!(new_delegations, expected);

    // Scenario 2: Validators already have uneven amounts of delegations
    // We just use the result from the previous scenario (112/111/111)
    let current_delegations = new_delegations;

    // Target amount per validator = (334 + 124) / 3 = 152
    // Remainer = 2
    // Alice:   152 + 1 - 112 = 41
    // Bob:     152 + 1 - 111 = 42
    // Charlie: 152 + 0 - 111 = 41
    let new_delegations = compute_delegations(Uint128::new(124), &current_delegations);
    let expected = vec![
        Delegation::new("alice", 41u128),
        Delegation::new("bob", 42u128),
        Delegation::new("charlie", 41u128),
    ];
    assert_eq!(new_delegations, expected,);

    // Scenario 3: A new validator was introduced
    let current_delegations = vec![
        Delegation::new("alice", 153u128),
        Delegation::new("bob", 153u128),
        Delegation::new("charlie", 152u128),
        Delegation::new("dave", 0u128),
    ];

    // Bond a small amount, say 15 uluna
    // Target: (153 + 153 + 152 + 0 + 15) / 4 = 118
    // Remainder: 1
    // Alice/Bob/Charlie get 0, Dave get all
    let new_delegations = compute_delegations(Uint128::new(15), &current_delegations);
    assert_eq!(new_delegations, vec![Delegation::new("dave", 15u128)],);

    // Bond a large amount, say 200 uluna
    // Target: (153 + 153 + 152 + 0 + 200) / 4 = 164
    // Remainder: 2
    // Alice:   164 + 1 - 153 = 12
    // Bob:     164 + 1 - 153 = 12
    // Charlie: 164 + 0 - 152 = 12
    // Dave:    164 + 0 - 0   = 164
    let new_delegations = compute_delegations(Uint128::new(200), &current_delegations);
    let expected = vec![
        Delegation::new("alice", 12u128),
        Delegation::new("bob", 12u128),
        Delegation::new("charlie", 12u128),
        Delegation::new("dave", 164u128),
    ];
    assert_eq!(new_delegations, expected);
}

#[test]
fn computing_undelegations() {
    let current_delegations = vec![
        Delegation::new("alice", 400u128),
        Delegation::new("bob", 300u128),
        Delegation::new("charlie", 200u128),
    ];

    // Target: (400 + 300 + 200 - 451) / 3 = 149
    // Remainder: 2
    // Alice:   400 - (149 + 1) = 250
    // Bob:     300 - (149 + 1) = 150
    // Charlie: 200 - (149 + 0) = 51
    let new_undelegations = compute_undelegations(Uint128::new(451), &current_delegations);
    let expected = vec![
        Undelegation::new("alice", 250u128),
        Undelegation::new("bob", 150u128),
        Undelegation::new("charlie", 51u128),
    ];
    assert_eq!(new_undelegations, expected);
}

#[test]
fn creating_delegation_msg() {
    let d = Delegation::new("alice", 12345u128);
    assert_eq!(
        d.to_cosmos_msg(),
        CosmosMsg::Staking(StakingMsg::Delegate {
            validator: String::from("alice"),
            amount: Coin::new(12345, "uluna"),
        }),
    );
}

#[test]
fn creating_undelegation_msg() {
    let ud = Undelegation::new("bob", 23456u128);
    assert_eq!(
        ud.to_cosmos_msg(),
        CosmosMsg::Staking(StakingMsg::Undelegate {
            validator: String::from("bob"),
            amount: Coin::new(23456, "uluna"),
        }),
    );
}

//--------------------------------------------------------------------------------------------------
// Coins
//--------------------------------------------------------------------------------------------------

#[test]
fn parsing_coin() {
    let coin = parse_coin("12345uatom").unwrap();
    assert_eq!(coin, Coin::new(12345, "uatom"));

    let coin =
        parse_coin("23456ibc/0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B")
            .unwrap();
    assert_eq!(
        coin,
        Coin::new(23456, "ibc/0471F1C4E7AFD3F07702BEF6DC365268D64570F7C1FDC98EA6098DD6DE59817B")
    );

    let err = parse_coin("69420").unwrap_err();
    assert_eq!(err, StdError::generic_err("failed to parse coin: 69420"));

    let err = parse_coin("ngmi").unwrap_err();
    assert_eq!(err, StdError::generic_err("Parsing u128: cannot parse integer from empty string"));
}

#[test]
fn parsing_coins() {
    let coins = Coins::from_str("").unwrap();
    assert_eq!(coins.0, vec![]);

    let coins = Coins::from_str("12345uatom").unwrap();
    assert_eq!(coins.0, vec![Coin::new(12345, "uatom")]);

    let coins = Coins::from_str("12345uatom,23456uluna").unwrap();
    assert_eq!(coins.0, vec![Coin::new(12345, "uatom"), Coin::new(23456, "uluna")]);
}

#[test]
fn adding_coins() {
    let mut coins = Coins(vec![]);

    coins = coins.add(&Coin::new(12345, "uatom")).unwrap();
    assert_eq!(coins.0, vec![Coin::new(12345, "uatom")]);

    coins = coins.add(&Coin::new(23456, "uluna")).unwrap();
    assert_eq!(coins.0, vec![Coin::new(12345, "uatom"), Coin::new(23456, "uluna")]);

    coins = coins.add_many(&Coins::from_str("76543uatom,69420uusd").unwrap()).unwrap();
    assert_eq!(
        coins.0,
        vec![Coin::new(88888, "uatom"), Coin::new(23456, "uluna"), Coin::new(69420, "uusd")]
    );
}

#[test]
fn receiving_funds() {
    let err = parse_received_fund(&[], "uluna").unwrap_err();
    assert_eq!(err, StdError::generic_err("must deposit exactly one coin; received 0"));

    let err = parse_received_fund(&[Coin::new(12345, "uatom"), Coin::new(23456, "uluna")], "uluna")
        .unwrap_err();
    assert_eq!(err, StdError::generic_err("must deposit exactly one coin; received 2"));

    let err = parse_received_fund(&[Coin::new(12345, "uatom")], "uluna").unwrap_err();
    assert_eq!(err, StdError::generic_err("expected uluna deposit, received uatom"));

    let err = parse_received_fund(&[Coin::new(0, "uluna")], "uluna").unwrap_err();
    assert_eq!(err, StdError::generic_err("deposit amount must be non-zero"));

    let amount = parse_received_fund(&[Coin::new(69420, "uluna")], "uluna").unwrap();
    assert_eq!(amount, Uint128::new(69420));
}
