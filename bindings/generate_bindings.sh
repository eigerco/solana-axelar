#!/bin/sh
cargo install native-to-anchor
program="axelar-solana-${1}"
native-to-anchor package ../solana/programs/${program} -o generated/temp -d anchor_lib/${program}.rs -k
# From generated temporary folder copy necessary files
cd generated/${program}
rm -rf program src idl.json
cp -r ../temp/${program}/program .
cp -r ../temp/${program}/src .
cp ../temp/${program}/idl.json .