import { Idl, Event, EventCoder } from "@coral-xyz/anchor";
import { IdlEvent } from "@coral-xyz/anchor/dist/cjs/idl";

export class AxelarSolanaMemoProgramEventsCoder implements EventCoder {
  constructor(_idl: Idl) {}

  decode<E extends IdlEvent = IdlEvent, T = Record<string, string>>(
    _log: string
  ): Event<E, T> | null {
    throw new Error("AxelarSolanaMemoProgram program does not have events");
  }
}
