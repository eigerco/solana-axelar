#!/bin/sh
cargo install shank-cli
shank idl -r solana/programs/axelar-solana-memo-program -o generated/axelar-solana-memo-program/idl
pnpm install
pnpm generate:clients
