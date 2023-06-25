#![cfg_attr(not(feature = "std"), no_std, no_main)]
use ink::env::*;

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature= "std", derive(scale_info::TypeInfo))]
pub enum Error {
    BalanceTooLow,
    AllowanceTooLow,
}

pub type Result<T> = core::result::Result<T, Error>;

type AccountId = <DefaultEnvironment as ::ink::env::Environment>::AccountId;
type Balance = <DefaultEnvironment as ::ink::env::Environment>::Balance;

#[ink::trait_definition]
pub trait TERC20 {
    /// Returns the total supply of the token
    #[ink(message)]
    fn total_supply(&self) -> Balance;

    /// Returns the balance of the owner.
    /// This represents the amount of tokens the owner has.
    #[ink(message)]
    fn balance_of(&self, who: AccountId) -> Balance;

    /// Returns the balance of the spender is still allowed to withdraw from the caller account.
    #[ink(message)]
    fn allowances_of(&self, spender: AccountId) -> Balance;

    /// Allows `spender` to withdraw from the caller's account multiple times, up to
    /// the `value` amount.
    #[ink(message)]
    fn approve(&mut self, spender: AccountId, value: Balance) -> Result<()>;

    /// Transfers the token from the caller to the given destination.
    #[ink(message)]
    fn transfer(&mut self, to: AccountId, value: Balance) -> Result<()>;

    /// Transfers `value` tokens on the behalf of `from` to the account `to`.
    /// Caller has to hold an approval with enough fund to spend from the sender
    #[ink(message)]
    fn transfer_from(&mut self, from: AccountId, to: AccountId, value: Balance) -> Result<()>;
}