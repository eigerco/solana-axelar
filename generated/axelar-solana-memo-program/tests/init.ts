import { axelarSolanaMemoProgramProgram } from "../src/program";

describe("Ping Memo Program", () => {
  const program = axelarSolanaMemoProgramProgram();

  it("Is initialized!", async () => {
    try {
        const tx = await program.methods.initialize(8).rpc();
        console.log("Your transaction signature", tx);
    } catch (error) {
        console.log(error);
    }
  });
});