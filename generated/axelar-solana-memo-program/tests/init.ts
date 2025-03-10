import { Keypair, PublicKey } from "@solana/web3.js";
import { axelarSolanaMemoProgramProgram, AXELAR_SOLANA_MEMO_PROGRAM_PROGRAM_ID } from "../src/program";
import { getKeypairFromFile } from "@solana-developers/node-helpers";

describe("Ping Memo Program", () => {
  const program = axelarSolanaMemoProgramProgram();

  it("Is initialized!", async () => {
    const payer = await getKeypairFromFile();
    const [gatewayRootPdaPublicKey, _] = PublicKey.findProgramAddressSync([], payer.publicKey);
    const bump = 8;
    let counterPdaPublicKey = PublicKey.createProgramAddressSync(
      [gatewayRootPdaPublicKey.toBuffer(), Buffer.from([bump])
    ], AXELAR_SOLANA_MEMO_PROGRAM_PROGRAM_ID);

    try {
        const tx = await program.methods.initialize(bump).accounts({
          payer: payer.publicKey,
          gatewayRootPda: gatewayRootPdaPublicKey,
          counterPda: counterPdaPublicKey
        }).rpc();
        console.log("Your transaction signature", tx);
    } catch (error) {
        console.log(error);
    }
    program.methods.processMemo("Test1").accounts({counterPda: counterPdaPublicKey}).rpc();
    program.methods.processMemo("Test2").accounts({counterPda: counterPdaPublicKey}).rpc();
  });
});