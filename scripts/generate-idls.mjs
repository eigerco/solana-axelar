#!/usr/bin/env zx
import 'zx/globals';
import { generateIdl } from '@metaplex-foundation/shank-js';
import { getCargo, getProgramFolders } from './utils.mjs';

const binaryInstallDir = path.join(__dirname, '..', '.cargo');

let programFolders = ['programs/axelar-solana-gateway'];
programFolders.forEach((folder) => {
  const cargo = getCargo(folder);
  const isShank = true;
  const programDir = path.join(__dirname, '..', folder);

  generateIdl({
    generator: isShank ? 'shank' : 'anchor',
    programName: cargo.package.name.replace(/-/g, '_'),
    programId: 'gtwLjHAsfKAR6GWB4hzTUAA1w4SDdFMKamtGA5ttMEe',
    idlDir: programDir,
    idlName: 'idl',
    programDir,
    binaryInstallDir,
  });
});
