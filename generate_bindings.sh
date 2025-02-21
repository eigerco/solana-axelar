#!/bin/sh
cargo install shank-cli
shank idl -r solana/programs/axelar-solana-gateway -o generated/idl
pnpm install
pnpm generate:clients
