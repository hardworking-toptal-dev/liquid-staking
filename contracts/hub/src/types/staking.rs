use cosmos_sdk_proto::cosmos::distribution::v1beta1::MsgWithdrawDelegatorReward;
use cosmos_sdk_proto::cosmos::staking::v1beta1::{MsgBeginRedelegate, MsgDelegate};
use cosmos_sdk_proto::cosmos::{base::v1beta1::Coin as SdkCoin, staking::v1beta1::MsgUndelegate};
use cosmwasm_std::{CosmosMsg, StdResult};

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct Delegation {
    pub validator: String,
    pub amount: u128,
    pub denom: String,
}
// "/liquidstaking.staking.v1beta1.MsgDelegate"

impl Delegation {
    pub fn new(validator: &str, amount: u128, denom: &str) -> Self {
        Self {
            validator: validator.to_string(),
            amount,
            denom: denom.to_string(),
        }
    }

    pub fn to_cosmos_msg(&self, delegator_address: String) -> StdResult<CosmosMsg> {
        crate::helpers::proto_encode(
            MsgDelegate {
                amount: Some(SdkCoin {
                    denom: self.denom.clone(),
                    amount: self.amount.to_string(),
                }),
                delegator_address,
                validator_address: self.validator.clone(),
            },
            "/liquidstaking.staking.v1beta1.MsgDelegate".to_string(),
        )
    }
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct Undelegation {
    pub validator: String,
    pub amount: u128,
    pub denom: String,
}

impl Undelegation {
    pub fn new(validator: &str, amount: u128, denom: &str) -> Self {
        Self {
            validator: validator.to_string(),
            amount,
            denom: denom.to_string(),
        }
    }

    pub fn to_cosmos_msg(&self, delegator_address: String) -> StdResult<CosmosMsg> {
        crate::helpers::proto_encode(
            MsgUndelegate {
                amount: Some(SdkCoin {
                    denom: self.denom.clone(),
                    amount: self.amount.to_string(),
                }),
                delegator_address,
                validator_address: self.validator.clone(),
            },
            "/liquidstaking.staking.v1beta1.MsgUndelegate".to_string(),
        )
    }
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct Redelegation {
    pub src: String,
    pub dst: String,
    pub amount: u128,
    pub denom: String,
}

impl Redelegation {
    pub fn new(src: &str, dst: &str, amount: u128, denom: &str) -> Self {
        Self {
            src: src.to_string(),
            dst: dst.to_string(),
            amount,
            denom: denom.into(),
        }
    }

    pub fn to_cosmos_msg(&self, delegator_address: String) -> StdResult<CosmosMsg> {
        crate::helpers::proto_encode(
            MsgBeginRedelegate {
                amount: Some(SdkCoin {
                    denom: self.denom.clone(),
                    amount: self.amount.to_string(),
                }),
                delegator_address,
                validator_src_address: self.src.clone(),
                validator_dst_address: self.dst.clone(),
            },
            "/liquidstaking.staking.v1beta1.MsgBeginRedelegate".to_string(),
        )
    }
}

#[derive(Clone)]
#[cfg_attr(test, derive(Debug, PartialEq, Eq))]
pub struct RewardWithdrawal {
    pub validator: String,
}

impl RewardWithdrawal {
    pub fn new(validator: &str) -> Self {
        Self {
            validator: validator.to_string(),
        }
    }

    pub fn to_cosmos_msg(&self, delegator_address: String) -> StdResult<CosmosMsg> {
        crate::helpers::proto_encode(
            MsgWithdrawDelegatorReward {
                delegator_address,
                validator_address: self.validator.clone(),
            },
            "/liquidstaking.distribution.v1beta1.MsgWithdrawDelegatorReward".to_string(),
        )
    }
}
