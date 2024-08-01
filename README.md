# trx-service

## Assumptions
- No persistence between runs required (using in-memory storage)
- Transaction ID is unique
- Only Deposits can be disputed. (with withdrawals the money already left the account, no point in dispute)
- It is possible to have a negative balance (chargebacks / disputes)
- No overflow prevention logic has been added.
    - Currently this cli is using an external crate that provides decimal with higher precision (recommended for financial applications)
- Silent errors (logged but do not panic. trx with errors are ignored) for
    - badly formatted lines
    - negative amounts
    - repeated transaction ids
    - referenced transactions that refer to different clients
    - all transactions in a locked account
- It is not possible to resolve a transaction that is not currently in dispute.
- Disputes are only allowed for:
    - Deposits, Withdrawals
    - Transactions that have in an Ok status, i.e. if the transaction is in dispute or a chargeback, cannot be set to Disputed.
## Future work
- Add overflow protection
- Add more integration tests and rework the structure to use a table-driven testing approach ([golang's table-driven tests](https://go.dev/wiki/TableDrivenTests))
- Add a maximum size to the buffered reader to prevent accidental or deliberate misuse.
- Add the concept of atomic operations. As we need to change some fields in an account, if a modification of these fields fails, all previous modifications should be reverted.
- Add more documentation following the standard defined in the rust book. 
- Add a more sophisticated way of handling CLI arguments and validations (see [CLAP](https://docs.rs/clap/latest/clap/)).
- Add benchmarks and performance tests. If there is a need to improve performance, consider implementing an async approach to take advantage of I/O operations.

## Compile

```sh
cargo build
```

## Run

```sh
cargo run -- file.csv
```

## Tests

```sh
cargo test
```

## Docs

```sh
cargo doc --open
```
