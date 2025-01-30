# Axelar Solana Gateway

> [!NOTE]
> Mandatory reading prerequisites:
> - [`Solidity Gateway reference implementation`](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/432449d7b330ec6edf5a8e0746644a253486ca87/contracts/gateway/INTEGRATION.md) developed by Axelar.
>
> Important Solana details are described in the docs:
> - [`Solana Account Model`](https://solana.com/docs/core/accounts)
> - [`Solana Transactions and Instructions`](https://solana.com/docs/core/transactions)
> - [`Solana CPI`](https://solana.com/docs/core/cpi)
> - [`Solana PDAs`](https://solana.com/docs/core/pda)
> 👆 a shorter-summary version is available [on Axelar Executable docs](../../crates/axelar-executable/README.md#solana-specific-rundown).

To integrate with the Axelar Solana Gateway, you are not exposed to be exposed to the inner workings and security mechanisms of the Gateway. 
- For receiving GMP messages from other chains, read [Axelar Executable docs](../../crates/axelar-executable/README.md).
- For sending messages to other chains, read [Sending messages from Solana](#sending-messages-from-solana).

## Sending messages from Solana

Here you can see the full flow of how a message gets proxied through the network when sending a message from Solana to any other chain:

![Solana to other chain](https://github.com/user-attachments/assets/61d9934e-221a-4858-be62-a70c5a12d21d)

For a destination contract to communicate with the Axelar Solana Gateway, it must make a CPI call to the Axelar Solana Gateway.
- On Solana, there is no such concept as `msg.sender` like there is in Solidity.
- On Solana `program_id`'s **cannot** be signers.
- On Solana, only PDAs can sign on behalf of a program. The only way for programs to send messages is to create PDAs that use [`invoke_signed()`](https://docs.rs/solana-cpi/latest/solana_cpi/fn.invoke_signed.html) and sign over the CPI call.
- The interface of `axelar_solana_gateway::GatewayInstruction::CallContract` instruction defines that the first account in the `accounts[]` must be the `program_id` that is sending the GMP payload.
- The sedond account is a "siging pda" meaining that a program must generate a PDA with specific paramters and sign the CPI call for `gateway.call_contract`; The presence of such signature acts as an authorizaton token, that allows the Gateway to interpret that the provided `program_id` is indeed the one that made the call and thus will use the `program_id` as the sender.

[Full fledged example](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-memo-program/src/processor.rs#L123-L157): Memo program that leverages a PDA for signing the `CallContract` CPI call.

| Gateway Instruction |  Use Case | Caveats |
| - | - | - |
| [CallContract](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/instructions.rs#L52-L67) | When you can create the data fully on-chain. Or When the data is small enough that you can fit it into tx arguments  | _None_ |
| [CallContractOffchainData](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/instructions.rs#L69-L85) | When the payload data cannot be generated on-chain or it does not fit into tx size limitations. This ix only requires the payload hash. The full payload is expected to be provided to the relayer directly | Wether the payload gets provided to the relayer before or after sending this ix is fully up to the relayer and not part of the Gateway spec. |

## Receiving messages from Solana

-- todo PDAs

### Expectations from the destination contract

### Work that the relayer is doing

