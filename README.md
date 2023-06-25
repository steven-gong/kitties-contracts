# letrentallin1-contracts
This is a set of smart contracts implemented by using ink! language to create a market of crypto kitties.

Development environment setup (Linux):

```
cargo install cargo-contract
rustup toolchain add nightly-2023-03-18
rustup target add wasm32-unknown-unknown --toolchain nightly-2023-03-18-x86_64-unknown-linux-gnu
rustup default nightly-2023-03-18-x86_64-unknown-linux-gnu
```

There are three smart contracts:
- Kitty Coin: A ERC 20 smart contract that can issue KittyCoin, manages account KittyCoin balances and KittyCoin transfer between accounts.
- Kitties: A ERC 721 nft smart contract that can manage kitties owned by accounts and transfer kitties between accounts.
- Kitty Market: A smart contract that can adopt and trade Kitties using KittyCoin.