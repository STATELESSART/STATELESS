// Initial implementation

// This contract will take funds received from a successful vote in STATELESS DAO 
// to transfer funds to all members.  When funds are received (NOTE FUTURE: automatic?)
// a call to this contract will trigger a payout of equal amounts to each DAO member

use near_env::{near_ext, near_log, PanicMessage};
use near_contract_standards::fungible_token::receiver::FungibleTokenReceiver;
use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::json_types::{ValidAccountId, U128, U64};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{
    env, ext_contract, log, near_bindgen, setup_alloc, AccountId, Balance, Gas, PanicOnDefault,
    PromiseOrValue,
};

setup_alloc!();

const BASE_GAS: Gas = 5_000_000_000_000;
const PROMISE_CALL: Gas = 5_000_000_000_000;
const GAS_FOR_DAO_TRANSFER: Gas = BASE_GAS + PROMISE_CALL;

const NO_DEPOSIT: Balance = 0;


// First work out how to get list of DAO members & count how many there are
// so that we can work out an even split

//TODO: Find a way to query this directly from Sputnik DAO
// For now, need to manually enter a list (sorry!!)

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct DAOpay {
    dao_account_id: AccountId,
}



// Defining cross-contract interface. This allows to create a new promise.
#[ext_contract(ext_self)]
pub trait ValueReturnTrait {
    fn value_please(&self, amount_to_return: String) -> PromiseOrValue<U128>;
}

// Have to repeat the same trait for our own implementation.
trait ValueReturnTrait {
    fn value_please(&self, amount_to_return: String) -> PromiseOrValue<U128>;
}

#[near_bindgen]
impl DAOpay {
    #[init]
    pub fn new(dao_account_id: ValidAccountId) -> Self {
        assert!(!env::state_exists(), "Already initialized");
        Self { dao_account_id: dao_account_id.into() }
    }
}

// Set up to work out the fractions
pub mod fraction {

    use super::CorePanics;
    use near_sdk::{
        borsh::{self, BorshDeserialize, BorshSerialize},
        serde::{Deserialize, Serialize},
        Balance,
    };
    use std::{fmt::Display, num::ParseIntError, str::FromStr, u128};

    uint::construct_uint! {
        /// 256-bit unsigned integer.
        struct U256(4);
    }

    /// Represents a number between `0` and `1`.
    /// It is meant to be used as percentage to calculate both fees and royalties.
    /// As with usual fractions, `den`ominator cannot be `0`.
    /// Morever, `num` must be less or equal than `den`.
    #[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Eq)]
    #[cfg_attr(not(target_arch = "wasm"), derive(Debug, Clone, Copy))]
    #[serde(crate = "near_sdk::serde")]
    pub struct Fraction {
        /// The *numerator* of this `Fraction`.
        pub num: u32,
        /// The *denominator* of this `Fraction`.
        pub den: u32,
    }

    impl Fraction {
        /// Checks the given `Fraction` is valid, *i.e.*,
        /// - Has a non-zero denominator, and
        /// - The `num` is less or equal than `den`ominator.
        pub fn check(&self) {
            if self.den == 0 {
                CorePanics::ZeroDenominatorFraction.panic();
            }
            if self.num > self.den {
                CorePanics::FractionGreaterThanOne.panic();
            }
        }

        /// Multiplies this `Fraction` by the given `value`.
        pub fn mult(&self, value: Balance) -> Balance {
            (U256::from(self.num) * U256::from(value) / U256::from(self.den)).as_u128()
        }
    }

    impl PartialEq for Fraction {
        fn eq(&self, other: &Self) -> bool {
            self.mult(u128::MAX) == other.mult(u128::MAX)
        }
    }

    impl PartialOrd for Fraction {
        fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
            self.mult(u128::MAX).partial_cmp(&other.mult(u128::MAX))
        }
    }

    impl Ord for Fraction {
        fn cmp(&self, other: &Self) -> std::cmp::Ordering {
            self.mult(u128::MAX).cmp(&other.mult(u128::MAX))
        }
    }

    impl Display for Fraction {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}/{}", self.num, self.den)
        }
    }

    #[cfg(not(target_arch = "wasm"))]
    impl FromStr for Fraction {
        type Err = ParseIntError;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            let parts = s.split("/").collect::<Vec<&str>>();
            Ok(Self { num: parts[0].parse::<u32>()?, den: parts[1].parse::<u32>()? })
        }
    }
}


/// Mapping from `AccountId`s to balance (in NEARs).
/// The balance indicates the amount a contract should pay when a transfer of funds is received.
pub type Payout = HashMap<AccountId, U128>;

/// Returns the sha256 of `value`.
pub fn crypto_hash(value: &String) -> CryptoHash {
    let mut hash = CryptoHash::default();
    hash.copy_from_slice(&env::sha256(value.as_bytes()));
    hash
}

/// Payouts is part of an ongoing (yet not settled) NEP spec:
/// <https://github.com/thor314/NEPs/blob/patch-5/specs/Standards/NonFungibleToken/payouts.md> 
/// therefore using some of the FT standard handling

    /// Query whom to pay out derived from list of STATELESS DAO members
    /// For example, if 4 members:
    ///
    /// - `split`: `25/100` (25%)
    ///
    /// Or, if 5 members:
    ///
    /// - `split`: `20/100` (20%)
    ///
    ///  Etc.

#[near_bindgen]
impl TokenReceiver for DAOpay {
    /// If given `msg: "send-my-money", immediately returns U128::From(0)
    /// Otherwise, makes a cross-contract call to own `value_please` function, passing `msg`
    /// value_please will attempt to parse `msg` as an integer and return a U128 version of it
    fn pay_on_transfer(
        &mut self,
        sender_id: ValidAccountId,
        amount: U128,
        msg: String,
    ) -> PromiseOrValue<U128> {
        // Verifying that we were called by fungible token contract that we expect.
        assert_eq!(
            &env::predecessor_account_id(),
            &self.dao_account_id,
            "Only supports the one token contract"
        );
        log!("in {} tokens from @{} pay_on_transfer, msg = {}", amount.0, sender_id.as_ref(), msg);
        match msg.as_str() {
            "take-my-money" => PromiseOrValue::Value(U128::from(0)),
            _ => {
                let prepaid_gas = env::prepaid_gas();
                let account_id = env::current_account_id();
                ext_self::value_please(
                    msg,
                    &account_id,
                    NO_DEPOSIT,
                    prepaid_gas - GAS_FOR_DAO_TRANSFER,
                )
                .into()
            }
        }
    }
}

#[near_bindgen]
impl ValueReturnTrait for DAOpay {
    fn value_please(&self, amount_to_return: String) -> PromiseOrValue<U128> {
        log!("in value_please, amount_to_return = {}", amount_to_return);
        let amount: Balance = amount_to_return.parse().expect("Not an integer");
        PromiseOrValue::Value(amount.into())
    }
}

#[near_ext]
#[ext_contract(self_callback)]
trait SelfCallback {
    fn make_payouts(&mut self);
}

#[near_log(skip_args, only_pub)]
#[near_bindgen]
impl SelfCallback for DAOpay {
    #[private]
    fn make_payouts(&mut self) {
        match env::promise_result(0) {
            PromiseResult::NotReady => unreachable!(),
            PromiseResult::Failed => unreachable!(),
            PromiseResult::Successful(value) => {
                if let Ok(payout) = serde_json::from_slice::<Payout>(&value) {
                    for (receiver_id, amount) in payout {
                        Promise::new(receiver_id).transfer(amount.0);
                    }
                } else {
                    unreachable!();
                }
            }
        }
    }
}
