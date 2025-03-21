import { Idl, StateCoder } from "@coral-xyz/anchor";

export class AxelarSolanaItsStateCoder implements StateCoder {
  constructor(_idl: Idl) {}

  encode<T = any>(_name: string, _account: T): Promise<Buffer> {
    throw new Error("AxelarSolanaIts does not have state");
  }
  decode<T = any>(_ix: Buffer): T {
    throw new Error("AxelarSolanaIts does not have state");
  }
}
