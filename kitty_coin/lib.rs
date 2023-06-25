#![cfg_attr(not(feature = "std"), no_std, no_main)]
pub use kitty_coin::{KittyCoin, KittyCoinRef};

#[ink::contract]
mod kitty_coin {
    use ink::storage::Mapping;
    use trait_erc20::{Error, Result, TERC20};

    #[ink(storage)]
    #[derive(Default)]
    pub struct KittyCoin {
        total_supply: Balance,
        balances: Mapping<AccountId, Balance>,
        allowances: Mapping<(AccountId, AccountId), Balance>,
    }

    #[ink(event)]
    pub struct Transfer {
        #[ink(topic)]
        from: Option<AccountId>,
        #[ink(topic)]
        to: Option<AccountId>,
        value: Balance,
    }

    #[ink(event)]
    pub struct Approval {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        spender: AccountId,
        value: Balance,
    }

    impl KittyCoin {
        #[ink(constructor)]
        pub fn new(total_supply: Balance) -> Self {
            let mut balances = Mapping::new();
            balances.insert(Self::env().caller(), &total_supply);

            Self::env().emit_event(Transfer {
                from: None,
                to: Some(Self::env().caller()),
                value: total_supply,
            });

            Self {
                total_supply,
                balances,
                ..Default::default()
            }
        }

        pub fn transfer_helper(
            &mut self,
            from: &AccountId,
            to: &AccountId,
            value: Balance,
        ) -> Result<()> {
            let balance_from = self.balance_of(*from);
            let balance_to = self.balance_of(*to);

            if value > balance_from {
                return Err(Error::BalanceTooLow);
            }

            self.balances.insert(from, &(balance_from - value));
            self.balances.insert(to, &(balance_to + value));

            self.env().emit_event(Transfer {
                from: Some(*from),
                to: Some(*to),
                value,
            });

            Ok(())
        }
    }

    impl TERC20 for KittyCoin {
        /// Returns the total supply of the token
        #[ink(message)]
        fn total_supply(&self) -> Balance {
            self.total_supply
        }

        /// Returns the balance of the owner.
        /// This represents the amount of tokens the owner has.
        #[ink(message)]
        fn balance_of(&self, who: AccountId) -> Balance {
            self.balances.get(&who).unwrap_or_default()
        }

        /// Returns the balance of the spender is still allowed to withdraw from the caller account.
        #[ink(message)]
        fn allowances_of(&self, spender: AccountId) -> Balance {
            let owner = self.env().caller();
            self.allowances.get(&(owner, spender)).unwrap_or_default()
        }

        /// Allows `spender` to withdraw from the caller's account multiple times, up to
        /// the `value` amount.
        #[ink(message)]
        fn approve(&mut self, spender: AccountId, value: Balance) -> Result<()> {
            let owner = self.env().caller();
            self.allowances.insert(&(owner, spender), &value);

            self.env().emit_event(Approval {
                owner,
                spender,
                value,
            });

            Ok(())
        }

        /// Transfers the token from the caller to the given destination.
        #[ink(message)]
        fn transfer(&mut self, to: AccountId, value: Balance) -> Result<()> {
            let sender = self.env().caller();
            self.transfer_helper(&sender, &to, value)
        }

        /// Transfers `value` tokens on the behalf of `from` to the account `to`.
        /// Caller has to hold an approval with enough fund to spend from the sender
        #[ink(message)]
        fn transfer_from(&mut self, from: AccountId, to: AccountId, value: Balance) -> Result<()> {
            let sender = self.env().caller();
            let allowance = self.allowances.get(&(from, sender)).unwrap_or_default();

            if allowance < value {
                return Err(Error::AllowanceTooLow);
            }

            self.allowances
                .insert(&(from, sender), &(allowance - value));

            self.transfer_helper(&from, &to, value)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        type Event = <KittyCoin as ::ink::reflect::ContractEventBase>::Type;
        #[ink::test]
        fn constructor_works() {
            let kitty_coin = KittyCoin::new(10_000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            assert_eq!(kitty_coin.total_supply(), 10_000);
            assert_eq!(kitty_coin.balance_of(accounts.alice), 10_000);

            let emitted_events = ink::env::test::recorded_events().collect::<Vec<_>>();
            let event = &emitted_events[0];
            let decoded =
                <Event as scale::Decode>::decode(&mut &event.data[..]).expect("decoded error");
            match decoded {
                Event::Transfer(Transfer { from, to, value }) => {
                    assert!(from.is_none(), "mint from error");
                    assert_eq!(to, Some(accounts.alice), "mint to error");
                    assert_eq!(value, 10_000, "mint value error");
                }
                _ => panic!("Transfer event not emitted"),
            }
        }

        #[ink::test]
        fn transfer_should_work() {
            let mut kitty_coin = KittyCoin::new(10_000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            let res = kitty_coin.transfer(accounts.bob, 12);
            assert!(res.is_ok());
            assert_eq!(kitty_coin.balance_of(accounts.alice), 10_000 - 12);
            assert_eq!(kitty_coin.balance_of(accounts.bob), 12);
        }

        #[ink::test]
        fn invalid_transfer_should_work() {
            let mut kitty_coin = KittyCoin::new(10_000);
            let accounts = ink::env::test::default_accounts::<ink::env::DefaultEnvironment>();
            ink::env::test::set_caller::<ink::env::DefaultEnvironment>(accounts.bob);

            let res = kitty_coin.transfer(accounts.charlie, 12);
            assert!(res.is_err());
            assert_eq!(res, Err(Error::BalanceTooLow));
        }
    }

    #[cfg(all(test, feature = "e2e-tests"))]
    mod e2e_tests {
        use super::*;
        use ink_e2e::build_message;

        type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

        #[ink_e2e::test]
        async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
            let total_supply = 1000;
            let constructor = KittyCoinRef::new(total_supply);

            let contract_account_id = client
                .instantiate("kitty_coin", &ink_e2e::alice(), constructor, 0, None)
                .await
                .expect("instantiate failed")
                .account_id;

            let alice_acc = ink_e2e::account_id(ink_e2e::AccountKeyring::Alice);
            let bob_acc = ink_e2e::account_id(ink_e2e::AccountKeyring::Bob);

            let transfer_msg = build_message::<KittyCoinRef>(contract_account_id.clone())
                .call(|kitty_coin| kitty_coin.transfer(bob_acc, 2));

            let res = client.call(&ink_e2e::alice(), transfer_msg, 0, None).await;

            assert!(res.is_ok());

            let balance_of_msg = build_message::<KittyCoinRef>(contract_account_id.clone())
                .call(|kitty_coin| kitty_coin.balance_of(alice_acc));

            let balance_of_alice = client
                .call_dry_run(&ink_e2e::alice(), &balance_of_msg, 0, None)
                .await;
            assert_eq!(balance_of_alice.return_value(), 998);

            Ok(())
        }
    }
}
