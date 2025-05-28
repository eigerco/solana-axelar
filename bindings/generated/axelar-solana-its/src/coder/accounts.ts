// @ts-nocheck
import * as B from "@native-to-anchor/buffer-layout";
import { AccountsCoder, Idl } from "@coral-xyz/anchor";
import { IdlTypeDef } from "@coral-xyz/anchor/dist/cjs/idl";

export class AxelarSolanaItsAccountsCoder<A extends string = string>
  implements AccountsCoder
{
  constructor(_idl: Idl) {}

  public async encode<T = any>(accountName: A, account: T): Promise<Buffer> {
    switch (accountName) {
      case "tokenManager": {
        const buffer = Buffer.alloc(106);
        const len = TOKEN_MANAGER_LAYOUT.encode(account, buffer);
        return buffer.slice(0, len);
      }
      default: {
        throw new Error(`Invalid account name: ${accountName}`);
      }
    }
  }

  public decode<T = any>(accountName: A, ix: Buffer): T {
    return this.decodeUnchecked(accountName, ix);
  }

  public decodeUnchecked<T = any>(accountName: A, ix: Buffer): T {
    switch (accountName) {
      case "tokenManager": {
        return decodeTokenManagerAccount(ix);
      }
      default: {
        throw new Error(`Invalid account name: ${accountName}`);
      }
    }
  }

  public memcmp(
    accountName: A,
    _appendData?: Buffer
  ): { dataSize?: number; offset?: number; bytes?: string } {
    switch (accountName) {
      case "tokenManager": {
        return {
          dataSize: 106,
        };
      }
      default: {
        throw new Error(`Invalid account name: ${accountName}`);
      }
    }
  }

  public size(idlAccount: IdlTypeDef): number {
    switch (idlAccount.name) {
      case "tokenManager": {
        return 106;
      }
      default: {
        throw new Error(`Invalid account name: ${idlAccount.name}`);
      }
    }
  }
}

function decodeTokenManagerAccount<T = any>(ix: Buffer): T {
  return TOKEN_MANAGER_LAYOUT.decode(ix) as T;
}

const TOKEN_MANAGER_LAYOUT: any = B.struct([
  ((p: string) => {
    const U = B.union(B.u8("discriminator"), null, p);
    U.addVariant(0, B.struct([]), "nativeInterchainToken");
    U.addVariant(1, B.struct([]), "mintBurnFrom");
    U.addVariant(2, B.struct([]), "lockUnlock");
    U.addVariant(3, B.struct([]), "lockUnlockFee");
    U.addVariant(4, B.struct([]), "mintBurn");
    return U;
  })("ty"),
  B.seq(B.u8(), 32, "tokenId"),
  B.publicKey("tokenAddress"),
  B.publicKey("associatedTokenAccount"),
  B.u64("flowLimit"),
  B.u8("bump"),
]);
