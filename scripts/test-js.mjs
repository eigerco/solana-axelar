#!/usr/bin/env zx
import 'zx/globals';

const workingDirectory = (await $`pwd`.quiet()).toString().trim();

// Build the client and run the tests.
cd(path.join(workingDirectory, 'generated', 'axelar-solana-memo-program'));
await $`pnpm install`;
await $`pnpm build`;
await $`pnpm test ${process.argv.slice(3)}`;