#![cfg_attr(not(feature = "std"), no_std, no_main)]
use ink::env::*;

#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode, Copy, Clone)]
#[cfg_attr(feature = "std", derive(scale_info::TypeInfo))]
pub enum Error {
    NotOwner,
    NotApproved,
    TokenExists,
    TokenNotFound,
    CannotInsert,
    CannotFetchValue,
    NotAllowed,
    CoinTransferFail,
}

pub type Result<T> = core::result::Result<T, Error>;

/// A token ID.
pub type KittyId = u32;
type AccountId = <DefaultEnvironment as ::ink::env::Environment>::AccountId;

#[ink::trait_definition]
pub trait TERC721 {
    /// Returns the balance of the owner.
    ///
    /// This represents the amount of unique tokens the owner has.
    #[ink(message)]
    fn balance_of(&self, owner: AccountId) -> u32;

    /// Returns the owner of the token.
    #[ink(message)]
    fn owner_of(&self, id: KittyId) -> Option<AccountId>;

    /// Returns the approved account ID for this token if any.
    #[ink(message)]
    fn get_approved(&self, id: KittyId) -> Option<AccountId>;

    /// Returns `true` if the operator is approved by the owner.
    #[ink(message)]
    fn is_approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool;

    /// Approves or disapproves the operator for all tokens of the caller.
    #[ink(message)]
    fn set_approval_for_all(&mut self, to: AccountId, approved: bool) -> Result<()>;

    /// Approves the account to transfer the specified token on behalf of the caller.
    #[ink(message)]
    fn approve(&mut self, to: AccountId, id: KittyId) -> Result<()>;

    /// Transfers the token from the caller to the given destination.
    #[ink(message)]
    fn transfer(&mut self, destination: AccountId, id: KittyId) -> Result<()>;

    /// Transfer approved or owned token.
    #[ink(message)]
    fn transfer_from(&mut self, from: AccountId, to: AccountId, id: KittyId) -> Result<()>;

    /// Creates a new token.
    #[ink(message)]
    fn mint(&mut self, id: KittyId) -> Result<()>;

    /// Deletes an existing token. Only the owner can burn the token.
    #[ink(message)]
    fn burn(&mut self, id: KittyId) -> Result<()>;
}
