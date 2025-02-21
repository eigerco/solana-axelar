# Instructions for working with bindings

Here are already prepared bindings which should work out of the box with the `memo-program` from this repository. Memo Program had some slight changes to make it work with the `native-to-anchor` binary. 

It has been tested on the local node. Initially, it has been built with `generate_bindings.sh` script that can be found at the root of the repo, but it also generates additional files which are unnecessary and therefore, removed. 

`native-to-anchor` created the `idl.json` file and from it, dummy lib file that represents `memo-program` instructions in `program\lib.rs`. Finally, it created `src` folder with typescript code.

On top of that, plain `Anchor.toml` has been provided here that has been copied from `hello_world` example and adjusted accordingly.

## Run Memo Program on local node

Start `solana-test-validator` in separte terminal. Go to `solana\programs\axelar-solana-memo-program` and build it with `cargo build-sbf`. From the root of the repo, run it with

```bash
solana program deploy solana/target/sbf-solana-solana/release/axelar_solana_memo_program.so --program-id solana/target/deploy/axelar_solana_memo_program-keypair.json
```

## Invoke the test

Before starting the test, install the dependencies:
 
```bash
pnpm install
```

And run the test:

```bash
anchor test --skip-local-validator
```

Following messages should appear:

```bash
transactionMessage: 'Transaction simulation failed: Error processing Instruction 0: insufficient account keys for instruction',
transactionLogs: [
    'Program 5fzGoxYfSbCvNR47tczb55wejYAyfeEhkE5f4Qnw4nKz invoke [1]',
    'Program log: Instruction: Native',
    'Program log: Instruction: Initialize',
    'Program 5fzGoxYfSbCvNR47tczb55wejYAyfeEhkE5f4Qnw4nKz consumed 1016 of 200000 compute units',
    'Program 5fzGoxYfSbCvNR47tczb55wejYAyfeEhkE5f4Qnw4nKz failed: insufficient account keys for instruction'
],
programErrorStack: ProgramErrorStack {
    stack: [
        [PublicKey [PublicKey(5fzGoxYfSbCvNR47tczb55wejYAyfeEhkE5f4Qnw4nKz)]]
    ]
}
```

It is because of the simplification of calling initialize in `memo-program`. This needs to be brought back to original state.

## Additional things

Bunch of typescript packages are defined in `package.json`. Some of them are probably not necessary.