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

### Messages where the payload data gets stored on-chain

Here you can see the full flow of how a message gets proxied through the network when sending a message from Solana to any other chain:

![image](https://github.com/user-attachments/assets/6f81d77c-6607-483b-a590-06394bed6b6e)

For a destination contract to communicate with the Axelar Solana Gateway, it must make a CPI call to the Axelar Solana Gateway.
- On Solana, there is no such concept as `msg.sender` like there is in Solidity.
- The interface of `axelar_solana_gateway::GatewayInstruction::CallContract` instruction defines that the first account in the `accounts[]` must be a signer, if the account is set as `signer` then the instruction will proceed and use the account pubkey as the origin of the message.
- On Solana `program_id`'s **cannot** be signers. This means that while a `program_id` must be a recipient of a GMP message (eg message from EVM to Solana), the `program_id` will **never** be the `sender` of a message; the relationship is not symetrical.
- The only way for programs to send messages is to create PDAs that use [`invoke_signed()`](https://docs.rs/solana-cpi/latest/solana_cpi/fn.invoke_signed.html) and sign over the CPI call.

[Full fledged example](https://github.com/eigerco/solana-axelar/blob/033bd17df32920eb6b57a0e6b8d3f82298b0c5ff/solana/programs/axelar-solana-memo-program/src/processor.rs#L123-L140): Memo program that leverages a PDA for signing the `CallContract` CPI call.

### Messages where the payload data is stored off-chain

todo
