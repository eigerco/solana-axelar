
# Relayer Discovery

  

The only information about an incoming contract call through axelar are

  

- Command Id: A unique identifier for the command approving the call, 32 bytes long.

- Source Chain: The name of the source chain, as a String

- Source Address: The caller, as a String.

- Destination Chain: “Sui” for the purposes of this document

- Destination Address: The destination, a Solana address.

  

The destination address will be the program id of a program. However there is no way for a relayer to know what they are supposed to call to get the call to be executed, since they don't know the list of accounts that need to be passed in.

  

## Relayer Discovery
Relayer discovery does not need to be a specific program on Solana. This is because programs can create accounts with predetermined addresses that 
Each `program_id` will be assigned a `transaction_pda` which is owned by the executable program and stores the transaction to be executed by the program. The `transaction_pda` should be derived by only a single seed: `keccak256('relayer-discovery-transaction') = 0xa57128349132c58c5700674195df81ef5ee89bc36f0e9676bae7e1479b7fcede`. This contents of this pda should strictly be the Borsh serialised data of the `RelayerTransaction` struct:

```rust
enum RelayerData {
	Bytes(Vec<u8>),
	Message(),
}
enum RelayerAccount {
	Account(AccountMeta),
	IncomingMessage(),
	MessagePayload(),
	Payer(Uint64)
}
struct RelayerInstruction {
	program: Pubkey,
	accounts: Vec<AccountMeta>,
	data: Vec<RelayerData>,
}

struct RelayerTransaction {
	is_final: bool,
	// solana supports a series of instructions in a single transaction,
	// do we want to support that or just use a single instruction?
	instructions: Vec<RelayerInstruction>,
}
```

Each `RelayerInstruction` can be converted into a regular `Instruction` by doing the following conversions:

- Each entry of `accounts` is either a hardcoded account, the `system_account`, the `incoming_message_pda`, the `message_payload_pda` or a `payer` account, with the specified `lamports`. These lamports are subtracted from the gas offered for the transaction. This last account is essential for executable to be able to write onto memory.
- `data` is converted to `Vec<u8>` by concatenating each of its elements together, with `Bytes` just being vectors, and `Message` being the Borsh serialized version of the [`Message`](https://github.com/eigerco/axelar-amplifier-solana/blob/next/solana/crates/axelar-solana-encoding/src/types/messages.rs#L53) to be executed. 

To figure out what to call the relayer needs to [obtain](https://solana.com/docs/rpc/http/getaccountinfo) this data and then run the following logic, [simulating](https://solana.com/docs/rpc/http/simulatetransaction) transactions when needed:
```
while(!relayer_transaction_is_final) {
	relayer_transaction = relayer_trnasaction.simulate().return_data.decode()
}
relayer_transaction.execute()
``` 

### An Example: Interchain Token Service receive Interchain Transfer without data
For an incoming message to the Interchain Token Service that needs to execute an inbound `InterchainTransfer` the executable needs to have prepared a PDA (once for all the messages, not every time) and the relayer needs to execute properly.
#### Executable
A one time call would have to be made to the InterchainTokenService that runs
```rust
let (transaction_pda, bump) = Pubkey::find_program_address(
	&[
		TRANSACTION_PDA_SEED, // 0xa57128349132c58c5700674195df81ef5ee89bc36f0e9676bae7e1479b7fcede
	],
	&crate::id(),
);
let transaction = RelayerTransaction {
	is_final: false,
	instructions: [ RelayerInstruction {
		program: crate::id(),
		accounts: [
			RelayerAccount::MessagePayload(),
		],
		data: [
			RelayerData::Bytes([
				// This should be a single byte that points to the build_transaction_1 function
				RELAYER_DISCOVERY_BUILD_TRANSACTION_1 
			]),
		],
	} ],
}

init_pda_raw_bytes(
	payer,
	transaction_pda,
	crate::id(),
	system_account,
	&to_vec(transaction),
	[
		TRANSACTION_PDA_SEED,
		bump,
	],
);
```
Then two functions need to be available to be called.
```rust

pub fun relayer_discovery_build_transaction_1( 
	accounts:  &'a [AccountInfo<'b>],
) {
	let payload: GMPPayload = get_incoming_payload_from_account(accounts[0]);
	
	match payload {
		GMPPayload::InterchainTransfer(transfer) => {
			let its_root_pda = crate::find_its_root_pda();
			let (token_manager_pda, _) = find_token_manager_pda(its_root_pda, transfer.token_id);
			RelayerTransaction {
				is_final: false,
				instructions: [ RelayerInstruction {
					program: crate::id(),
					accounts: [
						RelayerAccount::MessagePayload,
						RelayerAccount::Account(AccountMeta::new(token_manager_pda, false),
					],
					data: [
						RelayerData::Bytes([
							// This should be a single byte that points to the build_transaction_2 function
							RELAYER_DISCOVERY_BUILD_TRANSACTION_2
						]),
						RelayerData::Message(),
					],
				} ],
			}
		}
		// handle the rest of the cases here
	}
}

pub fun relayer_discovery_build_transaction_2( 
	accounts:  &'a [AccountInfo<'b>],
	message:  Message,
) {
	let payload: GMPPayload = get_incoming_payload_from_account(accounts[1]);
	
	let  command_id  =  command_id(&message.cc_id.chain, &message.cc_id.id);
	let (gateway_approved_message_signing_pda, _) = axelar_solana_gateway::get_validate_message_signing_pda(crate::ID, command_id);
	let  (gateway_root_pda, _)  =  axelar_solana_gateway::get_gateway_root_config_pda();

	let mut accounts = vec![
		RelayerAccount::Payer(INTERCHAIN_TRANSFER_COST),
		RelayerAccount::IncomingMessage(),
		RelayerAccount::MessagePayload(),
		RelayerAccount::Account(AccountMeta::new_readonly(gateway_approved_message_signing_pda, false)),
		RelayerAccount::Account(AccountMeta::new_readonly(gateway_root_pda, false)),
		RelayerAccount::Account(AccountMeta::new_readonly(axelar_solana_gateway::ID, false)),
	];
	
	let its_root_pda = crate::find_its_root_pda();
	let token_manager = TokenManager::load(accounts[1]);
	let token_mint = token_manager.token_address;
	let token_manager_ata = get_associated_token_address_with_program_id(
		&token_manager_pda,
		&token_mint,
		&spl_token_2022::ID,
	);
	let token_program = spl_associated_token_account::ID;
	
	accounts.append(vec![
		RelayerAccount::Account(AccountMeta::new_readonly(system_program::ID, false),
		RelayerAccount::Account(AccountMeta::new_readonly(its_root_pda, false),
		RelayerAccount::Account(AccountMeta::new(token_manager_pda, false),
		RelayerAccount::Account(AccountMeta::new(token_manager_ata, false),
		RelayerAccount::Account(AccountMeta::new(token_mint, false),
		RelayerAccount::Account(AccountMeta::new_readonly(spl_token_2022::ID, false),
		RelayerAccount::Account(AccountMeta::new_readonly(token_program, false),
		RelayerAccount::Account(AccountMeta::new_readonly(sysvar::rent::ID, false),	
	]);

	match payload {
		GMPPayload::InterchainTransfer(transfer) => {
			let  destination  =  Pubkey::new_from_array(
				transfer.destination_address.into(),
			);
			let  destination_ata  =  get_associated_token_address_with_program_id(
				&destination,
				&token_mint,
				&token_program,
			);
			
			accounts.append(vec![
				RelayerAccount::Account(AccountMeta::new(destination, false),
				RelayerAccount::Account(AccountMeta::new(destination_ata, false),
			] ); 
			
			if transfer.data.is_empty() {
				RelayerTransaction {
					is_final: true,
					instructions: [ RelayerInstruction {
						program: crate::id(),
						accounts,
						data: [
							RelayerData::Bytes([
								// This should be a single byte that points to the proccess_execute function
								EXECUTE
							]),
							RelayerData::Message()
						],
					} ],
				}
			} else {
				// Handle the case of having to execute as well, which is even more complicated, requiring a different entry point in the destination program so that it can tell us what the accounts it needs are.
			}
		}
		// match the rest of the cases here
	}
}
```
#### Relayer
The relayer needs to make a few different calls to an node:
1. Call `getAccountInfo` for the  `transaction_pda` initiated by the `InterchainTokenService` 
2.  Parse the response into a `RelayerTransaction` object. Convert this into a regular transaction (`relayer_discovery_build_transction_1`). Call `simulateTransaction` with this information since the `RelayerTransaction` was not marked as `final`.
3. Parse the response into another `RelayerTransaction` object, which can be used to call `simulateTransaction` once more (`relayer_discovery_build_transaction_2`), since the result was not `final` once more.
4. The response now can be converted to a `final` `RelayerTransaction`. It requires that they `payer` is passed as a signer and some funds are requested. If the gas covers the amount requested alongside what would be needed for the execution (another `simulateTransaction` is needed to determine this, but it is not instrumental for this example) then make sure the `payer` has exactly the funds requested (to ensure loss of additional funds).
5. Finally execute the transaction, passing the properly funded `payer`.

### Explanation
The reason why we need two calls instead of one is because to find the `token_mint` account from the payload we need to first calculate the `token_manager_pda`, then load its contents to find the `token_mint`. The first call asks the relayer to call once more providing the `token_manager` account.