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


| PDA name | descriptoin | users | notes | owner |
| - | - | - | - | - |
| [CallContract](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/lib.rs#L312-L317) | This acts only as a signing PDA, never initialized; Gives permission to the destination program to call `CallContract` on the Gateway | Destination program will craft this when making the CPI call to the Gateway | Emulates `msg.sender` from Solidity | Destination program |

[Full fledged example](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-memo-program/src/processor.rs#L123-L157): Memo program that leverages a PDA for signing the `CallContract` CPI call.

| Gateway Instruction |  Use Case | Caveats |
| - | - | - |
| [Call Contract](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/instructions.rs#L52-L67) | When you can create the data fully on-chain. Or When the data is small enough that you can fit it into tx arguments  | Even if you can generate all the data on-chain, the solana tx log is limited to 10kb. And if your program logs more than that, there won't be any error on the tx levle. The log will be truncated and the message will malformed. **Use with caution.**  |
| [Call Contract Offchain Data](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/instructions.rs#L69-L85) | When the payload data cannot be generated on-chain or it does not fit into tx size limitations. This ix only requires the payload hash. The full payload is expected to be provided to the relayer directly | Wether the payload gets provided to the relayer before or after sending this ix is fully up to the relayer and not part of the Gateway spec. |

### Axelar network steps

After the relayer sends the message to Amplifier API, that's when Axelar network and ampd perform all the validations

![image](https://github.com/user-attachments/assets/e7a137e7-6545-4161-be7e-91ec9d6223a5)

- Relevant ampd code is located [here, axelar-amplifier/solana/ampd](https://github.com/eigerco/axelar-amplifier/tree/solana/ampd)
- `ampd` will query the Solana RPC network for a given tx hash (in Solanas case it's actually the tx signature, which is 64 bytes)
  - retrieve the logs, parse them using [`gateway-event-stack` crate](https://github.com/eigerco/solana-axelar/tree/next/solana/crates/gateway-event-stack) to parse the logs, and then try to find an event at the given index. If the event exists and the contents match, then `ampd` will produce signatures for the rest of the Axelar network to consume


## Receiving messages on Solana

Receiving messages on Solana is a multitude more complex than sending messages. There are a couple of PDAs that are involved in the process.

![image](https://github.com/user-attachments/assets/43e0ac3b-04e9-4d76-9075-8b325aec278b)

| PDA name | descriptoin | users | notes | owner |
| - | - | - | - | - |
| [Gateway Config](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/state/config.rs) | Tracks all the information about the Gateway, the verifier set epoch, verifier set hashes, verifier rotation delays, etc.  | This PDA is present in all the public interfaces on the Gateway. Relayer, and every contract is expected to interact with it | | Gateway |
| [Verifier Set Tracker](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/state/verifier_set_tracker.rs) | Tracks information about an individual verifier set | Relayer, when rotationg verifier sets; Relayer, when approving messages; | Solana does not have built-in infinitie size hash maps as storage variables, using PDA for each verifier set entry allows us to ensure that duplicate verifier sets never get created | Gateway |
| [Signtautre Verification Session](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/state/signature_verification_pda.rs) | Tracks that all the signatures for a given payload batch get verified | Relayer uses this in the multi-tx message approval process, where each signature from a verifier is sent individually to the Gateway for verification | | Gateway |
| [Incoming Message](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/state/incoming_message.rs) | Tracks the state of an individual GMP message (executed / approved + metadata). | Relayer - after all the signatures have been approved, each GMP message must be initialized individually as well, relayer takes care of that. Destination program will receive this PDA in its `execute` flow when receiving the payload | | Gateway |
| [Message Payload](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/state/message_payload.rs) | Contains the raw payload of a message. Limited of up to 10kb. Directly linked to an `IncomingMessage` PDA. | Relayer will upload the raw payload to a PDA and after message execution (or failure of execution) will close the PDA regaining all the funds. Destination program will receive this PDA in its `execute` flow. | Solana tx size limitation prevents sending large payolads directly on chain, thus the payload is stored directly on-chain | Gateway; Relayer that created it can also close it |
| [Validate Call](https://github.com/eigerco/solana-axelar/blob/bf3351013ccf5061aaa1195411e2430c67250ec8/solana/programs/axelar-solana-gateway/src/lib.rs#L286-L291) | This acts only as a signing PDA, never initialized; Gives permission to the destination program to set `IncomingMessage` status to `executed`; | Destination program will craft this when making the CPI call to the Gateway | Emulates `msg.sender` from Solidity | Destination program |


### Expectations from the destination contract

For a third party developer to build an integrartion with the Axelar Solana Gateway, the only expectation is for the contract to implement [`axelar-executable`](../../crates/axelar-executable/README.md) interface. This allows the Relayer to have a known interfacte that it can send messages to, after they've been approved on the Gateway




