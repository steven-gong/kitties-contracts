#![cfg_attr(not(feature = "std"), no_std, no_main)]

#[ink::contract]
mod kitty_market {
    use ink::{prelude::vec::Vec, storage::Mapping};
    use trait_erc721::{TERC721, KittyId};
    use trait_erc20::{TERC20};

    #[ink(storage)]
    pub struct KittyMarket {
        kitties_contract_account: AccountId,
        kitty_coin: ink::contract_ref!(TERC20),
        kitties: ink::contract_ref!(TERC721),
        /// A mapping from kitty listed for sale to its price.
        kitties_for_sale: Mapping<KittyId, u128>,
        /// A vector of kitty ids listed for sale.
        kitty_ids_for_sale: Vec<KittyId>,
        /// A list of kitties needs adoption
        kitties_for_adoption: Vec<KittyId>,
        minted_count: u32,
    }

    #[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
    #[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
    pub enum Error {
        /// Kitty does not have any owner
        NoOwner,
        /// Kitty can be listed for sale only by its owner
        NotOwner,
        /// Kitty is not for sale
        NotForSale,
        /// Kitty is already listed for sale
        AlreadyListedForSale,
        /// Kitty is not for adoption
        NotForAdoption,
        /// Kitty is already listed for adoption
        AlreadyListedForAdoption,
        /// Price cannot be zero
        PriceIsZero,
        /// Failed to make kitty coin payment
        CoinTransferFail,
        /// Failed to change kitty ownership
        OwnershipTransferFail,
        /// Owned kitties count not found
        OwnedKittiesCountNotFound,
        /// Kitties contract account failed to gain the permission to transfer kitty to future adopter
        ListAdoptNotApproved,
        /// Kitties contract account failed to gain the permission to transfer kitty to future buyer
        ListSaleNotApproved,
    }

    pub type Result<T> = core::result::Result<T, Error>;

    #[ink(event)]
    pub struct ListedForAdoption {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        kitty_id: KittyId,
    }

    #[ink(event)]
    pub struct Adopted {
        #[ink(topic)]
        adopter: AccountId,
        #[ink(topic)]
        kitty_id: KittyId,
    }

    #[ink(event)]
    pub struct ListedForSale {
        #[ink(topic)]
        owner: AccountId,
        #[ink(topic)]
        kitty_id: KittyId,
        price: u128,
    }

    #[ink(event)]
    pub struct Sold {
        #[ink(topic)]
        seller: AccountId,
        #[ink(topic)]
        buyer: AccountId,
        #[ink(topic)]
        kitty_id: KittyId,
        price: u128,
    }

    impl KittyMarket {
        #[ink(constructor)]
        pub fn new(kitties: AccountId, kitty_coin: AccountId) -> Self {
            Self {
                kitties_contract_account: kitties.clone(),
                kitty_coin: kitty_coin.into(),
                kitties: kitties.into(),                
                kitties_for_sale: Mapping::new(),
                kitty_ids_for_sale: Vec::new(),
                kitties_for_adoption: Vec::new(),
                minted_count: 0,                
            }
        }

        /// Returns list of kitties waiting to be adopted
        #[ink(message)]
        pub fn adoption_list(&self) -> Vec<KittyId> {
            self.kitties_for_adoption.clone()
        }

        /// Returns list of kitties for sale
        #[ink(message)]
        pub fn kitties_for_sale(&self) -> Vec<(KittyId, u128)> {
            self.kitty_ids_for_sale.iter().map(|&id| (id, self.kitties_for_sale.get(&id).unwrap())).collect()
        }

        /// List a kitty for adoption
        #[ink(message)]
        pub fn list_for_adoption(&mut self, kitty_id: KittyId) -> Result<()> {
            let caller = self.env().caller();
            let owner = self.kitties.owner_of(kitty_id);

            if owner != Some(caller) {
                return Err(Error::NotOwner);
            }
            let owner = owner.expect("owner is valid");

            if self.kitties_for_adoption.contains(&kitty_id) {
                return Err(Error::AlreadyListedForAdoption);
            }

            // TODO: Fix approve call
            let list_adopt_result = self.kitties.approve(self.kitties_contract_account, kitty_id);
            if list_adopt_result.is_err() {
                return Err(Error::ListAdoptNotApproved);
            }

            self.kitties_for_adoption.push(kitty_id);
            self.kitty_ids_for_sale.retain(|&id| id != kitty_id);

            Self::env().emit_event(ListedForAdoption {
                owner,
                kitty_id,
            });

            Ok(())
        }

        /// Adopt a kitty
        #[ink(message)]
        pub fn adopt(&mut self, kitty_id: KittyId) -> Result<()> {
            let adopter = self.env().caller();

            if !self.kitties_for_adoption.contains(&kitty_id) {
                return Err(Error::NotForAdoption);
            }

            let owner = self.kitties.owner_of(kitty_id).expect("owner is valid");
            
            let ownership_transfer_result = self.kitties.transfer_from(owner, adopter, kitty_id); 
            if ownership_transfer_result.is_err() {
                return Err(Error::OwnershipTransferFail);
            }

            self.kitties_for_adoption.retain(|&id| id != kitty_id);

            Self::env().emit_event(Adopted {
                adopter,
                kitty_id,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn list_for_sale(&mut self, kitty_id: KittyId, price: u128) -> Result<()> {
            let caller = self.env().caller();
            let owner = self.kitties.owner_of(kitty_id);

            if owner != Some(caller) {
                return Err(Error::NotOwner);
            }
            let owner = owner.expect("owner in valid");

            if price == 0 {
                return Err(Error::PriceIsZero);
            }

            if self.kitties_for_sale.contains(kitty_id) {
                return Err(Error::AlreadyListedForSale);
            }

            // TODO: Fix approve call
            let approve_result = self.kitties.approve(self.kitties_contract_account, kitty_id);
            if approve_result.is_err() {
                return Err(Error::ListSaleNotApproved);
            }

            self.kitties_for_sale.insert(kitty_id, &price);
            self.kitty_ids_for_sale.push(kitty_id);
            self.kitties_for_adoption.retain(|&id| id != kitty_id);

            Self::env().emit_event(ListedForSale {
                owner,
                kitty_id,
                price,
            });

            Ok(())
        }

        #[ink(message)]
        pub fn buy(&mut self, kitty_id: KittyId) -> Result<()> {
            let buyer = self.env().caller();

            // Check if the kitty is listed for sale
            if !self.kitties_for_sale.contains(kitty_id) {
                return Err(Error::NotForSale);
            }
            let price = self.kitties_for_sale.get(kitty_id).expect("kitty price should be valid");
            
            let maybe_owner = self.kitties.owner_of(kitty_id);
            if maybe_owner.is_none() {
                return Err(Error::NoOwner);
            }
            let seller = maybe_owner.expect("owner should be valid");

            let payment_result = self.kitty_coin.transfer_from(buyer, seller, price);
            if payment_result.is_err() {
                return Err(Error::CoinTransferFail);
            }

            // TODO: Remove this, change kitty_id from u32 to a random value, and update kitties logic
            // self.minted_count += 1;
            // let mint_res = self.kitties.mint(self.minted_count);
            // if mint_res.is_err() {
            //     return Err(Error::MintFail);
            // }

            let ownership_transfer_result = self.kitties.transfer_from(seller, buyer, kitty_id);
            if ownership_transfer_result.is_err() {
                return Err(Error::OwnershipTransferFail);
            }

            self.kitties_for_sale.remove(kitty_id);
            self.kitty_ids_for_sale.retain(|&id| id != kitty_id);

            Self::env().emit_event(Sold {
                seller,
                buyer,
                kitty_id,
                price,
            });

            Ok(())
        }

        // TODO: Add a call to unlist kitty from adoption list
        // TODO: Add a call to unlist kitty from sale list
    }

    // #[cfg(test)]
    // mod tests {
    //     use super::*;
 
    //     /// We test if the default constructor does its job.
    //     #[ink::test]
    //     fn default_works() {
    //         let kitties = Kitties::new();
    //         let kitty_coin = KittyCoin::new(10_000);
    //         let kitty_market = KittyMarket::new(kitties, kitty_coin);
    //         assert_eq!(kitty_market.get(), false);
    //     }

    //     // /// We test a simple use case of our contract.
    //     // #[ink::test]
    //     // fn list_for_adoption_should_work() {
    //     //     let mut kitty_market = KittyMarket::new(false);
    //     //     assert_eq!(kitty_market.get(), false);
    //     //     kitty_market.flip();
    //     //     assert_eq!(kitty_market.get(), true);
    //     // }
    // }


    // /// This is how you'd write end-to-end (E2E) or integration tests for ink! contracts.
    // ///
    // /// When running these you need to make sure that you:
    // /// - Compile the tests with the `e2e-tests` feature flag enabled (`--features e2e-tests`)
    // /// - Are running a Substrate node which contains `pallet-contracts` in the background
    // #[cfg(all(test, feature = "e2e-tests"))]
    // mod e2e_tests {
    //     /// Imports all the definitions from the outer scope so we can use them here.
    //     use super::*;

    //     /// A helper function used for calling contract messages.
    //     use ink_e2e::build_message;

    //     /// The End-to-End test `Result` type.
    //     type E2EResult<T> = std::result::Result<T, Box<dyn std::error::Error>>;

    //     /// We test that we can upload and instantiate the contract using its default constructor.
    //     #[ink_e2e::test]
    //     async fn default_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
    //         // Given
    //         let constructor = KittyMarketRef::default();

    //         // When
    //         let contract_account_id = client
    //             .instantiate("kitty_market", &ink_e2e::alice(), constructor, 0, None)
    //             .await
    //             .expect("instantiate failed")
    //             .account_id;

    //         // Then
    //         let get = build_message::<KittyMarketRef>(contract_account_id.clone())
    //             .call(|kitty_market| kitty_market.get());
    //         let get_result = client.call_dry_run(&ink_e2e::alice(), &get, 0, None).await;
    //         assert!(matches!(get_result.return_value(), false));

    //         Ok(())
    //     }

    //     /// We test that we can read and write a value from the on-chain contract contract.
    //     #[ink_e2e::test]
    //     async fn it_works(mut client: ink_e2e::Client<C, E>) -> E2EResult<()> {
    //         // Given
    //         let constructor = KittyMarketRef::new(false);
    //         let contract_account_id = client
    //             .instantiate("kitty_market", &ink_e2e::bob(), constructor, 0, None)
    //             .await
    //             .expect("instantiate failed")
    //             .account_id;

    //         let get = build_message::<KittyMarketRef>(contract_account_id.clone())
    //             .call(|kitty_market| kitty_market.get());
    //         let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
    //         assert!(matches!(get_result.return_value(), false));

    //         // When
    //         let flip = build_message::<KittyMarketRef>(contract_account_id.clone())
    //             .call(|kitty_market| kitty_market.flip());
    //         let _flip_result = client
    //             .call(&ink_e2e::bob(), flip, 0, None)
    //             .await
    //             .expect("flip failed");

    //         // Then
    //         let get = build_message::<KittyMarketRef>(contract_account_id.clone())
    //             .call(|kitty_market| kitty_market.get());
    //         let get_result = client.call_dry_run(&ink_e2e::bob(), &get, 0, None).await;
    //         assert!(matches!(get_result.return_value(), true));

    //         Ok(())
    //     }
    // }
}
