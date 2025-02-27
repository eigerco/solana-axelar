#!/bin/sh
cargo install shank-cli
shank idl -r solana/programs/axelar-solana-memo-program -o generated/idl
pnpm install
pnpm generate:clients
