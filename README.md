# letrentallin1-contracts
This is a set of smart contracts implemented by using ink! language to create a market of crypto kitties.

Development environment setup (Linux):

```
cargo install cargo-contract
rustup toolchain add nightly-2023-03-18
rustup target add wasm32-unknown-unknown --toolchain nightly-2023-03-18-x86_64-unknown-linux-gnu
rustup default nightly-2023-03-18-x86_64-unknown-linux-gnu
```