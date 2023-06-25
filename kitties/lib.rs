//! # Kitties
//!
//! This is an Kitties ERC-721 Kitty implementation.
//!
//! ## Warning
//!
//! This contract is an *example*. It is neither audited nor endorsed for production use.
//! Do **not** rely on it to keep anything of value secure.
//!
//! ## Overview
//!
//! This contract demonstrates how to build non-fungible or unique tokens using ink!.
//!
//! ## Error Handling
//!
//! Any function that modifies the state returns a `Result` type and does not changes the
//! state if the `Error` occurs.
//! The errors are defined as an `enum` type. Any other error or invariant violation
//! triggers a panic and therefore rolls back the transaction.
//!
//! ## Kitty Management
//!
//! After creating a new kitty, the function caller becomes the owner.
//! A kitty can be created, transferred, or destroyed.
//!
//! Kitty owners can assign other accounts for transferring specific kitties on their
//! behalf. It is also possible to authorize an operator (higher rights) for another
//! account to handle kitties.
//!
//! ### Kitty Creation
//!
//! Kitty creation start by calling the `mint(&mut self, id: u32)` function.
//! The kitty owner becomes the function caller. The Kitty ID needs to be specified
//! as the argument on this function call.
//!
//! ### Kitty Transfer
//!
//! Transfers may be initiated by:
//! - The owner of a kitty
//! - The approved address of a kitty
//! - An authorized operator of the current owner of a kitty
//!
//! The kitty owner can transfer a kitty by calling the `transfer` or `transfer_from`
//! functions. An approved address can make a kitty transfer by calling the
//! `transfer_from` function. Operators can transfer kitties on another account's behalf or
//! can approve a kitty transfer for a different account.
//!
//! ### Kitty Removal
//!
//! Kitty token can be destroyed by burning them. Only the kitty token owner is allowed to burn a
//! kitty token.

#![cfg_attr(not(feature = "std"), no_std, no_main)]
pub use kitties::{Kitties, KittiesRef};

#[ink::contract]
mod kitties {
    use ink::storage::Mapping;
    use trait_erc721::{Error, Result, KittyId, TERC721};
    use trait_erc20::TERC20;

    #[ink(storage)]
    pub struct Kitties {
        /// Mapping from kitty to owner.
        kitty_owner: Mapping<KittyId, AccountId>,
        /// Mapping from kitty to approvals users.
        token_approvals: Mapping<KittyId, AccountId>,
        /// Mapping from owner to number of owned kitty.
        owned_kitties_count: Mapping<AccountId, u32>,
        /// Mapping from owner to operator approvals.
        operator_approvals: Mapping<(AccountId, AccountId), ()>,
        /// Kitty coin contract reference
        acceptable_erc20: ink::contract_ref!(TERC20),
        /// Price for minting a kitty
        mint_price: u128,
    }

    /// Event emitted when a kitty transfer occurs.
    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        #[ink(topic)]
        id: KittyId,
    }

    /// Event emitted when a kitty approve occurs.
    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        from: AccountId,
        #[ink(topic)]
        to: AccountId,
        #[ink(topic)]
        id: KittyId,
    }

    /// Event emitted when an operator is enabled or disabled for an owner.
    /// The operator can manage all NFTs of the owner.
    #[ink(event)]
    pub struct ApprovalForAll {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        operator: AccountId,
        approved: bool,
    }

    impl Kitties {
        /// Creates a new Kitties ERC-721 token contract.
        #[ink(constructor)]
        pub fn new(erc20: AccountId, mint_price: u128) -> Self {
            Self {
                acceptable_erc20: erc20.into(),
                mint_price,
                kitty_owner: Mapping::new(),
                token_approvals: Mapping::new(),
                owned_kitties_count: Mapping::new(),
                operator_approvals: Mapping::new(),
            }           
        }

        /// Transfers kitty `id` `from` the sender to the `to` `AccountId`.
        pub fn transfer_token_from(
            &mut self,
            from: &AccountId,
            to: &AccountId,
            id: KittyId,
        ) -> Result<()> {
            let caller = self.env().caller();
            if !self.exists(id) {
                return Err(Error::TokenNotFound);
            };
            if !self.approved_or_owner(Some(caller), id) {
                return Err(Error::NotApproved);
            };
            self.clear_approval(id);
            self.remove_token_from(from, id)?;
            self.add_token_to(to, id)?;
            self.env().emit_event(Transfer {
                from: Some(*from),
                to: Some(*to),
                id,
            });
            Ok(())
        }

        /// Removes kitty `id` from the owner.
        pub fn remove_token_from(&mut self, from: &AccountId, id: KittyId) -> Result<()> {
            let Self {
                kitty_owner,
                owned_kitties_count,
                ..
            } = self;

            if !kitty_owner.contains(id) {
                return Err(Error::TokenNotFound);
            }

            let count = owned_kitties_count
                .get(from)
                .map(|c| c - 1)
                .ok_or(Error::CannotFetchValue)?;
            owned_kitties_count.insert(from, &count);
            kitty_owner.remove(id);

            Ok(())
        }

        /// Adds the kitty `id` to the `to` AccountID.
        pub fn add_token_to(&mut self, to: &AccountId, id: KittyId) -> Result<()> {
            let Self {
                kitty_owner,
                owned_kitties_count,
                ..
            } = self;

            if kitty_owner.contains(id) {
                return Err(Error::TokenExists);
            }

            if *to == AccountId::from([0x0; 32]) {
                return Err(Error::NotAllowed);
            };

            let count = owned_kitties_count.get(to).map(|c| c + 1).unwrap_or(1);

            owned_kitties_count.insert(to, &count);
            kitty_owner.insert(id, to);

            Ok(())
        }

        /// Approves or disapproves the operator to transfer all kitties of the caller.
        pub fn approve_for_all(&mut self, to: AccountId, approved: bool) -> Result<()> {
            let caller = self.env().caller();
            if to == caller {
                return Err(Error::NotAllowed);
            }
            self.env().emit_event(ApprovalForAll {
                owner: caller,
                operator: to,
                approved,
            });

            if approved {
                self.operator_approvals.insert((&caller, &to), &());
            } else {
                self.operator_approvals.remove((&caller, &to));
            }

            Ok(())
        }

        /// Approve the passed `AccountId` to transfer the specified kitty on behalf of
        /// the message's sender.
        pub fn approve_for(&mut self, to: &AccountId, id: KittyId) -> Result<()> {
            let caller = self.env().caller();
            let owner = self.owner_of(id);
            if !(owner == Some(caller)
                || self.approved_for_all(owner.expect("Error with AccountId"), caller))
            {
                return Err(Error::NotAllowed);
            };

            if *to == AccountId::from([0x0; 32]) {
                return Err(Error::NotAllowed);
            };

            if self.token_approvals.contains(id) {
                return Err(Error::CannotInsert);
            } else {
                self.token_approvals.insert(id, to);
            }

            self.env().emit_event(Approval {
                from: caller,
                to: *to,
                id,
            });

            Ok(())
        }

        /// Removes existing approval from kitty `id`.
        pub fn clear_approval(&mut self, id: KittyId) {
            self.token_approvals.remove(id);
        }

        // Returns the total number of kitties from an account.
        pub fn balance_of_or_zero(&self, of: &AccountId) -> u32 {
            self.owned_kitties_count.get(of).unwrap_or(0)
        }

        /// Gets an operator on other Account's behalf.
        pub fn approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
            self.operator_approvals.contains((&owner, &operator))
        }

        /// Returns true if the `AccountId` `from` is the owner of kitty `id`
        /// or it has been approved on behalf of the kitty `id` owner.
        pub fn approved_or_owner(&self, from: Option<AccountId>, id: KittyId) -> bool {
            let owner = self.owner_of(id);
            from != Some(AccountId::from([0x0; 32]))
                && (from == owner
                    || from == self.token_approvals.get(id)
                    || self.approved_for_all(
                        owner.expect("Error with AccountId"),
                        from.expect("Error with AccountId"),
                    ))
        }

        /// Returns true if kitty `id` exists or false if it does not.
        pub fn exists(&self, id: KittyId) -> bool {
            self.kitty_owner.contains(id)
        }
    }

    impl TERC721 for Kitties {
        /// Returns the balance of the owner.
        ///
        /// This represents the amount of unique kitties the owner has.
        #[ink(message)]
        fn balance_of(&self, owner: AccountId) -> u32 {
            self.balance_of_or_zero(&owner)
        }

        /// Returns the owner of the kitty.
        #[ink(message)]
        fn owner_of(&self, id: KittyId) -> Option<AccountId> {
            self.kitty_owner.get(id)
        }

        /// Returns the approved account ID for this kitty if any.
        #[ink(message)]
        fn get_approved(&self, id: KittyId) -> Option<AccountId> {
            self.token_approvals.get(id)
        }

        /// Returns `true` if the operator is approved by the owner.
        #[ink(message)]
        fn is_approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
            self.approved_for_all(owner, operator)
        }

        /// Approves or disapproves the operator for all kitties of the caller.
        #[ink(message)]
        fn set_approval_for_all(&mut self, to: AccountId, approved: bool) -> Result<()> {
            self.approve_for_all(to, approved)?;
            Ok(())
        }

        /// Approves the account to transfer the specified kitty on behalf of the caller.
        #[ink(message)]
        fn approve(&mut self, to: AccountId, id: KittyId) -> Result<()> {
            self.approve_for(&to, id)?;
            Ok(())
        }

        /// Transfers the kitty from the caller to the given destination.
        #[ink(message)]
        fn transfer(&mut self, destination: AccountId, id: KittyId) -> Result<()> {
            let caller = self.env().caller();
            self.transfer_token_from(&caller, &destination, id)?;
            Ok(())
        }

        /// Transfer approved or owned kitty.
        #[ink(message)]
        fn transfer_from(&mut self, from: AccountId, to: AccountId, id: KittyId) -> Result<()> {
            self.transfer_token_from(&from, &to, id)?;
            Ok(())
        }

        /// Creates a new kitty.
        #[ink(message)]
        fn mint(&mut self, id: KittyId) -> Result<()> {
            let caller = self.env().caller();
            let kitties_account = self.env().account_id().into();

            let payment_result = self.acceptable_erc20.transfer_from(caller, kitties_account, self.mint_price);
            if payment_result.is_err() {
                return Err(Error::CoinTransferFail);
            }

            self.add_token_to(&caller, id)?;

            self.env().emit_event(Transfer {
                from: Some(AccountId::from([0x0; 32])),
                to: Some(caller),
                id,
            });
            Ok(())
        }

        /// Deletes an existing kitty. Only the owner can burn the kitty.
        #[ink(message)]
        fn burn(&mut self, id: KittyId) -> Result<()> {
            let caller = self.env().caller();
            let Self {
                kitty_owner,
                owned_kitties_count,
                ..
            } = self;

            let owner = kitty_owner.get(id).ok_or(Error::TokenNotFound)?;
            if owner != caller {
                return Err(Error::NotOwner);
            };

            let count = owned_kitties_count
                .get(caller)
                .map(|c| c - 1)
                .ok_or(Error::CannotFetchValue)?;
            owned_kitties_count.insert(caller, &count);
            kitty_owner.remove(id);

            self.env().emit_event(Transfer {
                from: Some(caller),
                to: Some(AccountId::from([0x0; 32])),
                id,
            });

            Ok(())
        }
    }

    /// Unit tests
    #[cfg(test)]
    mod tests {
        /// Imports all the definitions from the outer scope so we can use them here.
        use super::*;

        #[ink::test]
        fn mint_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut kitties = Kitties::new();
            // Kitty 1 does not exists.
            assert_eq!(kitties.owner_of(1), None);
            // Alice does not owns kitties.
            assert_eq!(kitties.balance_of(accounts.alice), 0);
            // Create kitty Id 1.
            assert_eq!(kitties.mint(1), Ok(()));
            // Alice owns 1 kitty.
            assert_eq!(kitties.balance_of(accounts.alice), 1);
        }

        #[ink::test]
        fn mint_existing_should_fail() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut kitties = Kitties::new();
            // Create kitty Id 1.
            assert_eq!(kitties.mint(1), Ok(()));
            // The first Transfer event takes place
            assert_eq!(1, ink::env::test::recorded_events().count());
            // Alice owns 1 kitty.
            assert_eq!(kitties.balance_of(accounts.alice), 1);
            // Alice owns kitty Id 1.
            assert_eq!(kitties.owner_of(1), Some(accounts.alice));
            // Cannot create  kitty Id if it exists.
            // Bob cannot own kitty Id 1.
            assert_eq!(kitties.mint(1), Err(Error::TokenExists));
        }

        #[ink::test]
        fn transfer_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut kitties = Kitties::new();
            // Create kitty Id 1 for Alice
            assert_eq!(kitties.mint(1), Ok(()));
            // Alice owns kitty 1
            assert_eq!(kitties.balance_of(accounts.alice), 1);
            // Bob does not owns any kitty
            assert_eq!(kitties.balance_of(accounts.bob), 0);
            // The first Transfer event takes place
            assert_eq!(1, ink::env::test::recorded_events().count());
            // Alice transfers kitty 1 to Bob
            assert_eq!(kitties.transfer(accounts.bob, 1), Ok(()));
            // The second Transfer event takes place
            assert_eq!(2, ink::env::test::recorded_events().count());
            // Bob owns kitty 1
            assert_eq!(kitties.balance_of(accounts.bob), 1);
        }

        #[ink::test]
        fn invalid_transfer_should_fail() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut kitties = Kitties::new();
            // Transfer kitty fails if it does not exists.
            assert_eq!(
                kitties.transfer(accounts.bob, 2),
                Err(Error::TokenNotFound)
            );
            // Kitty Id 2 does not exists.
            assert_eq!(kitties.owner_of(2), None);
            // Create kitty Id 2.
            assert_eq!(kitties.mint(2), Ok(()));
            // Alice owns 1 kitty.
            assert_eq!(kitties.balance_of(accounts.alice), 1);
            // Kitty Id 2 is owned by Alice.
            assert_eq!(kitties.owner_of(2), Some(accounts.alice));
            // Set Bob as caller
            set_caller(accounts.bob);
            // Bob cannot transfer not owned kitties.
            assert_eq!(
                kitties.transfer(accounts.eve, 2),
                Err(Error::NotApproved)
            );
        }

        #[ink::test]
        fn approved_transfer_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut kitties = Kitties::new();
            // Create kitty Id 1.
            assert_eq!(kitties.mint(1), Ok(()));
            // Kitty Id 1 is owned by Alice.
            assert_eq!(kitties.owner_of(1), Some(accounts.alice));
            // Approve kitty Id 1 transfer for Bob on behalf of Alice.
            assert_eq!(kitties.approve(accounts.bob, 1), Ok(()));
            // Set Bob as caller
            set_caller(accounts.bob);
            // Bob transfers kitty Id 1 from Alice to Eve.
            assert_eq!(
                kitties.transfer_from(accounts.alice, accounts.eve, 1),
                Ok(())
            );
            // KittyId 3 is owned by Eve.
            assert_eq!(kitties.owner_of(1), Some(accounts.eve));
            // Alice does not owns kitties.
            assert_eq!(kitties.balance_of(accounts.alice), 0);
            // Bob does not owns kitties.
            assert_eq!(kitties.balance_of(accounts.bob), 0);
            // Eve owns 1 kitty.
            assert_eq!(kitties.balance_of(accounts.eve), 1);
        }

        #[ink::test]
        fn approved_for_all_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut kitties = Kitties::new();
            // Create kitty Id 1.
            assert_eq!(kitties.mint(1), Ok(()));
            // Create kitty Id 2.
            assert_eq!(kitties.mint(2), Ok(()));
            // Alice owns 2 kitties.
            assert_eq!(kitties.balance_of(accounts.alice), 2);
            // Approve kitty Id 1 transfer for Bob on behalf of Alice.
            assert_eq!(
                kitties.set_approval_for_all(accounts.bob, true),
                Ok(())
            );
            // Bob is an approved operator for Alice
            assert!(kitties.is_approved_for_all(accounts.alice, accounts.bob));
            // Set Bob as caller
            set_caller(accounts.bob);
            // Bob transfers kitty Id 1 from Alice to Eve.
            assert_eq!(
                kitties.transfer_from(accounts.alice, accounts.eve, 1),
                Ok(())
            );
            // KittyId 1 is owned by Eve.
            assert_eq!(kitties.owner_of(1), Some(accounts.eve));
            // Alice owns 1 kitty.
            assert_eq!(kitties.balance_of(accounts.alice), 1);
            // Bob transfers kitty Id 2 from Alice to Eve.
            assert_eq!(
                kitties.transfer_from(accounts.alice, accounts.eve, 2),
                Ok(())
            );
            // Bob does not own kitties.
            assert_eq!(kitties.balance_of(accounts.bob), 0);
            // Eve owns 2 kitties.
            assert_eq!(kitties.balance_of(accounts.eve), 2);
            // Remove operator approval for Bob on behalf of Alice.
            set_caller(accounts.alice);
            assert_eq!(
                kitties.set_approval_for_all(accounts.bob, false),
                Ok(())
            );
            // Bob is not an approved operator for Alice.
            assert!(!kitties.is_approved_for_all(accounts.alice, accounts.bob));
        }

        #[ink::test]
        fn not_approved_transfer_should_fail() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut kitties = Kitties::new();
            // Create kitty Id 1.
            assert_eq!(kitties.mint(1), Ok(()));
            // Alice owns 1 kitty.
            assert_eq!(kitties.balance_of(accounts.alice), 1);
            // Bob does not owns kitties.
            assert_eq!(kitties.balance_of(accounts.bob), 0);
            // Eve does not owns kitties.
            assert_eq!(kitties.balance_of(accounts.eve), 0);
            // Set Eve as caller
            set_caller(accounts.eve);
            // Eve is not an approved operator by Alice.
            assert_eq!(
                kitties.transfer_from(accounts.alice, accounts.frank, 1),
                Err(Error::NotApproved)
            );
            // Alice owns 1 kitty.
            assert_eq!(kitties.balance_of(accounts.alice), 1);
            // Bob does not owns kitties.
            assert_eq!(kitties.balance_of(accounts.bob), 0);
            // Eve does not owns kitties.
            assert_eq!(kitties.balance_of(accounts.eve), 0);
        }

        #[ink::test]
        fn burn_works() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut kitties = Kitties::new();
            // Create kitty Id 1 for Alice
            assert_eq!(kitties.mint(1), Ok(()));
            // Alice owns 1 kitty.
            assert_eq!(kitties.balance_of(accounts.alice), 1);
            // Alice owns kitty Id 1.
            assert_eq!(kitties.owner_of(1), Some(accounts.alice));
            // Destroy kitty Id 1.
            assert_eq!(kitties.burn(1), Ok(()));
            // Alice does not owns kitties.
            assert_eq!(kitties.balance_of(accounts.alice), 0);
            // Kitty Id 1 does not exists
            assert_eq!(kitties.owner_of(1), None);
        }

        #[ink::test]
        fn burn_fails_token_not_found() {
            // Create a new contract instance.
            let mut kitties = Kitties::new();
            // Try burning a non existent kitty
            assert_eq!(kitties.burn(1), Err(Error::TokenNotFound));
        }

        #[ink::test]
        fn burn_fails_not_owner() {
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            // Create a new contract instance.
            let mut kitties = Kitties::new();
            // Create kitty Id 1 for Alice
            assert_eq!(kitties.mint(1), Ok(()));
            // Try burning this kitty with a different account
            set_caller(accounts.eve);
            assert_eq!(kitties.burn(1), Err(Error::NotOwner));
        }

        fn set_caller(sender: AccountId) {
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(sender);
        }
    }
}
