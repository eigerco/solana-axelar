#!/bin/sh
cargo install native-to-anchor
native-to-anchor package solana/programs/axelar-solana-memo-program -k -o generated/temp
# From generated temporary folder copy necessary files
cd generated/axelar-solana-memo-program
rm -rf program src idl.json
cp -r ../temp/axelar-solana-memo-program/program .
cp -r ../temp/axelar-solana-memo-program/src .
cp ../temp/axelar-solana-memo-program/idl.json .
