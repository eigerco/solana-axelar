#!/usr/bin/env zx
import 'zx/globals';
import * as c from 'codama';
import { rootNodeFromAnchor } from '@codama/nodes-from-anchor';
import { renderVisitor as renderJavaScriptVisitor } from '@codama/renderers-js';
import { renderVisitor as renderRustVisitor } from '@codama/renderers-rust';
import { getAllProgramIdls } from './utils.mjs';

// Instead of calling getAllProgramIdls() from utils.mjs, hardcoded idl value because it fails for rust code 
// It has to contain Cargo.toml, not sure if it should be before or after
// It is not necessary for now, JS/TS bindings are the necessary ones
let allProgramIdls = ['/home/ivan/Development/solana_projects/solana_bindings/program/idl.json'];
const [idl, ...additionalIdls] = allProgramIdls.map((idl) =>
  rootNodeFromAnchor(require(idl))
);
const codama = c.createFromRoot(idl, additionalIdls);

// Optional: Update programs.
codama.update(
  c.updateProgramsVisitor({
    solanaProgramMyProgram: { name: 'myProgram' },
  })
);

// Optional: Update accounts.
codama.update(
  c.updateAccountsVisitor({
    counter: {
      seeds: [
        c.constantPdaSeedNodeFromString('utf8', 'counter'),
        c.variablePdaSeedNode(
          'authority',
          c.publicKeyTypeNode(),
          'The authority of the counter account'
        ),
      ],
    },
  })
);

// Optional: Update instructions.
codama.update(
  c.updateInstructionsVisitor({
    create: {
      byteDeltas: [c.instructionByteDeltaNode(c.accountLinkNode('counter'))],
      accounts: {
        counter: { defaultValue: c.pdaValueNode('counter') },
        payer: { defaultValue: c.accountValueNode('authority') },
      },
    },
    increment: {
      accounts: {
        counter: { defaultValue: c.pdaValueNode('counter') },
      },
      arguments: {
        amount: { defaultValue: c.noneValueNode() },
      },
    },
  })
);

// Optional: Set account discriminators.
const key = (name) => ({ field: 'key', value: c.enumValueNode('Key', name) });
codama.update(
  c.setAccountDiscriminatorFromFieldVisitor({
    counter: key('counter'),
  })
);

// Only necessary to render JavaScript code
const jsClient = path.join(__dirname, '..', 'clients', 'js');
codama.accept(
  renderJavaScriptVisitor(path.join(jsClient, 'src', 'generated'), {
    prettierOptions: require(path.join(jsClient, '.prettierrc.json')),
  })
);

// Leave out Rust rendering for now
// const rustClient = path.join(__dirname, '..', 'clients', 'rust');
// codama.accept(
//   renderRustVisitor(path.join(rustClient, 'src', 'generated'), {
//     formatCode: true,
//     crateFolder: rustClient,
//   })
// );
