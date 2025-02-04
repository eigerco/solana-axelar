# Solana-Axelar Interoperability

This repository contains the integration work between Solana and Axelar, enabling seamless cross-chain communication. The project includes General Message Passing (GMP) contracts and other Axelra core components.

## Table of Contents

- [Repository contents](#repository-contents)
  - [Solana contracts](#solana-contracts)
  - [EVM Smart Contracts](#evm-smart-contracts)
  - [Offchain microservices](#offchain-microservices)
- [Getting Started](#getting-started)
  - [Prerequisites](#prerequisites)
  - [Installation](#installation)
  - [Usage](#usage)
- [Contributing](#contributing)
- [License](#license)


## Repository contents

![image](https://github.com/user-attachments/assets/88008f1c-4096-4248-87b2-128b65cb8e41)

The Solana-Axelar integrations contains on-chain and off-chain components.

### Solana contracts
- [**Gateway**](solana/programs/axelar-solana-gateway/README.md): The core contract responsible for authenticating GMP messages.
- [**Gas Service**](solana/programs/axelar-solana-gas-service/README.md): Used for gas payments for the relayer.
- [**Interchain Token Service**](solana/programs/axelar-solana-its/README.md): Bridge tokens between chains.
- [**Multicall**](solana/programs/axelar-solana-multicall): Execute multiple actions from a single GMP message.
- [**Governance**](solana/programs/axelar-solana-governance/README.md): The governing entity over on-chain programs, responsible for program upgrades
- [**Memo**](solana/programs/axelar-solana-memo-program): An example program that is able to send and receive GMP messages.


#### Utility crates
- [**Axelar Executable**](solana/crates/axelar-executable/README.md): A set of libraries & interfaces that the destination program (3rd party integration) must implement.
- [**Axelar Solana Encoding**](solana/crates/axelar-solana-encoding/README.md): Encoding used by the Multisig Prover to encode the data in a way that the relayer & the Solana Gateway can interpret.
- [**Gateway Event Stack**](solana/crates/gateway-event-stack): Used by the Relayer to parse events coming from the Gas Service & the Gateway.

### EVM Smart Contracts
- [**Axelar Memo**](evm-contracts/src/AxelarMemo.sol): A counterpart of the `axelar-solana-memo` program that acts a an example program used to send GMP messages back and forth Solana.
- [**Axelar Solana Multi Call**](evm-contracts/src/AxelarSolanaMultiCall.sol): An example contract used to showcase how to compose multicall payloads for Solana
- [**Solana Gateway Payload**](evm-contracts/src/ExampleEncoder.sol): A Solditiy library that can create Solana-specific GMP payloads.


## Related Repositories

- [**Solana Relayer**](https://github.com/eigerco/axelar-solana-relayer): The off-chain entity that will route your messages to and from Solana.
- [**Relayer Core**](https://github.com/eigerco/axelar-relayer-core): All Axelar-related relayer infrastructure. Used as a core buidling block for the Solana Relayer. Also used by the Axelar-Starknet and Axlelar-Aleo relayers.
- [**Multisig Prover**](https://github.com/eigerco/axelar-amplifier/tree/add-multisig-prover-sol-logic/contracts/multisig-prover): The entity on the Axelar chain that is responsible for encoding the data for the Relayer and the Solana Gateway
- [**Utility Scripts**](https://github.com/eigerco/solana-axelar-scripts): Deployment scripts; GMP testing scripts and other utilities.


## Getting Started

### Prerequisites

- [List of prerequisites, e.g., Rust, Solana CLI, Axelar SDK, etc.]

### Installation

```bash
# Clone the repo (& init submodules)
git clone --recurse-submodules <repo url>

# if the repo has already been cloned you need to fetch the submodules
git submodule update --init --recursive
```

### Usage

- [Instructions on how to run the project, deploy contracts, etc.]

## Contributing

- [Guidelines for contributing to the repo, including pull request processes, coding standards, etc.]

## License

- [Details about the licensing of the project]

## About [Eiger](https://www.eiger.co)

We are engineers. We contribute to various ecosystems by building low level implementations and core components. We work on several Axelar and Solana projects and believe that connecting these two is a very important goal to achieve cross-chain execution.

Contact us at hello@eiger.co
Follow us on [X/Twitter](https://x.com/eiger_co)
