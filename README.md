# UTXO Miden

A demo project built on [MidenVM](https://0xpolygonmiden.github.io/miden-vm/intro/main.html). The purpose of this project is to show how state updates can be handled in MidenVM.

## Rust implementation

The UTXO semantics are implemented in Rust without MidenVM. This allows testing that the ZK implementation matches the Rust implementation (and the Rust implementation is easier to verify as correct because it is a higher level language than Miden Assembly). You can use the CLI tool to interact with Rust implementation of the UTXO protocol.

First compile the project:
```shell
cargo build --release
```

All the Rust implementation functionality is available under the `no-zk` command of the CLI:

```
$ ./target/release/utxo-miden-cli no-zk --help
Usage: utxo-miden-cli no-zk <COMMAND>

Commands:
  generate-key-pair    Generate a new key pair to use for signing UTXO transactions
  create-state         Create a new state with a single UTXO in it
  process-transaction  Send a transaction, updating the state. A key file must exist for the signer (one can be created via `GenerateKeyPair`). The transaction is specified as a JSON file (see `SerializedTransaction`)
  help                 Print this message or the help of the given subcommand(s)

Options:
  -h, --help  Print help
```

### Example execution

You can generate new key-pairs for signing transactions using the `generate-key-pair` command. However, for the purpose of this example, two keys are pre-generated and included in the `example` directory. You can use these keys to proceed through an example execution of the UTXO protocol.

First generate a state with a single UTXO owned by one of the keys:

```
$ ./target/release/utxo-miden-cli no-zk create-state --owner 0xda51ad197710bafc3192226e859c8b29a2b1757dafcda157a0a293a8e392517c --value 0xff00000000000000

State root = 0x7f9f3674fed4e65332aa719365de3bfbaa7c6042db99dc956dba117415d2bc97
State written to "example/state.json"
```

Then you can execute `example/tx_1.json` which splits that one UTXO into two, one owned by each key:

```
./target/release/utxo-miden-cli no-zk process-transaction --signer 0xda51ad197710bafc3192226e859c8b29a2b1757dafcda157a0a293a8e392517c --tx-path ./example/tx_1.json

State root = 0xcc1c4b56de4304be11b09aff1f753baa145e4fc2e53646a96fa6b95ea40660d2
State written to "example/state.json"
```

The state is overridden and now contains two new UTXOs. If you try to run the command again you will get an error because the UTXO used by `tx_1.json` has already been consumed:

```
$ ./target/release/utxo-miden-cli no-zk process-transaction --signer 0xda51ad197710bafc3192226e859c8b29a2b1757dafcda157a0a293a8e392517c --tx-path ./example/tx_1.json
Error: Error processing transaction

Caused by:
    UnknownUtxoHash
```

There is a second transaction `example/tx_2.json` which sends the value back to the original owner. It must be signed by the other key because otherwise the signature check fails:

```
$ ./target/release/utxo-miden-cli no-zk process-transaction --signer 0xda51ad197710bafc3192226e859c8b29a2b1757dafcda157a0a293a8e392517c --tx-path ./example/tx_2.json
Error: Error processing transaction

Caused by:
    InvalidTransaction(InvalidSignature)
```

Using the correct signer we successfully perform the state transition:

```
$ ./target/release/utxo-miden-cli no-zk process-transaction --signer 0x496d5921189e0f6a49b64c90a62286a47381bd63641ef9d847f7b9fc917b68f8 --tx-path ./example/tx_2.json

State root = 0x03a72f5706e8e5ce3d0f4550efb32d5eec62aa9b21ee4933bcde457de3139e71
State written to "example/state.json"
```

The new state still has two UTXOs, but now they are both owned by the same key.

The final transaction, `example/tx_3.json`, burns the second UTXO so that only 1 remains again (the state root is different though because we have lost `0x0f` value since we initially generated the state):

```
$ ./target/release/utxo-miden-cli no-zk process-transaction --signer 0xda51ad197710bafc3192226e859c8b29a2b1757dafcda157a0a293a8e392517c --tx-path ./example/tx_3.json
State root = 0x1c7dfcae5078d5536c4931d6c4bb63eaf56dcc29d27f3ec3495149b1c3451a49
State written to "example/state.json"
```
