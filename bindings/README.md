# Instructions for working with bindings

Here are already prepared bindings which should work out of the box with the `memo-program` and `gateway` from this repository. Both, `memo-program` and `gateway` had some slight changes to make it work with the `native-to-anchor` binary. 

It has been tested on the local node.

`native-to-anchor` created the `idl.json` file and from it, lib file that represents `memo-program` and a `src` folder with typescript code.

On top of that, plain `Anchor.toml` has been provided here that has been copied from `hello_world` example and adjusted accordingly.

## NO NEED TO RUN `generate_bindings.sh`

Because of the complexity of our programs, custom made `anchor` files are already prepared in `anchor_lib/` and the `native-to-anchor` binary that is called within the script, uses these files.

Initially, this folder has been built with `generate_bindings.sh` script, but scrpt also generates additional files which are unnecessary and are stored in the `temp` folder. There is no need to run the script, because the bindings are already prepared.

In case that script should need to be run, right now it can only be run in two ways: to generate `memo-program` or `gateway`. Script can be run in following ways:

```bash
./generate_bindings.sh memo-program
```

Or:

```bash
./generate_bindings.sh gateway
```

In that way, it would rewrite the already created bindings. If no changes were made in the `anchor_lib/` folder, then no changes should be in the `generated/` folder.

## Run Memo Program on local node

Start `solana-test-validator` in separate terminal. 

From the `root` of the repository build `memo-program`:

```bash
cd solana/programs/axelar-solana-memo-program && cargo build-sbf
```

And deploy it:


```bash
cd ../../../ && solana program deploy solana/target/sbf-solana-solana/release/axelar_solana_memo_program.so --program-id solana/target/deploy/axelar_solana_memo_program-keypair.json
```

## Run Gateway on local node

From the `root` of the repository build `gateway`:

```bash
cd solana/programs/axelar-solana-gateway && cargo build-sbf 
```

And deploy it:


```bash
cd ../../../ && solana program deploy solana/target/sbf-solana-solana/release/axelar_solana_gateway.so --program-id solana/target/deploy/axelar_solana_gateway-keypair.json
```

## Invoke the test

Before starting the test, change folder to `bindings/generated/`:

```bash
cd bindings/generated
```

Install the dependencies:
 
```bash
pnpm install
```

And run the test:

```bash
anchor test --skip-local-validator
```

Following messages should appear:

```bash
Ping Gateway
Test OK: Program throws error, but data is properly sent through bindings.
    ✔ ApproveMessage (76ms)
Test OK: Program throws error, but data is properly sent through bindings.
    ✔ RotateSigners
Test OK: Program throws error, but data is properly sent through bindings.
    ✔ CallContract
Test OK: Program throws error, but data is properly sent through bindings.
    ✔ CallContractOffchainData
Test OK: Program throws error, but data is properly sent through bindings.
    ✔ InitializeConfig
Test OK: Program throws error, but data is properly sent through bindings.
    ✔ InitializePayloadVerificationSession
Test OK: Program throws error, but data is properly sent through bindings.
    ✔ VerifySignature
Test OK: Program throws error, but data is properly sent through bindings.
    ✔ InitializeMessagePayload
Test OK: Program throws error, but data is properly sent through bindings.
    ✔ WriteMessagePayload
Test OK: Program throws error, but data is properly sent through bindings.
    ✔ CommitMessagePayload
Test OK: Program throws error, but data is properly sent through bindings.
    ✔ CloseMessagePayload
Test OK: Program throws error, but data is properly sent through bindings.
    ✔ ValidateMessage
Test OK: Program throws error, but data is properly sent through bindings.
    ✔ TransferOperatorship

  Ping Memo Program
Initializing failed, probably it has been already initialized. Skipping...
    ✔ Is initialized!
```

## Additional things

Bunch of typescript packages are defined in `generated/package.json`. Some of them are probably not necessary.