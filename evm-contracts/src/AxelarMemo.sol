// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.19;

import {AbiSolanaGatewayPayload, SolanaGatewayPayload, SolanaAccountRepr} from "./SolanaGatewayPayload.sol";
import {AxelarExecutable} from "cgp-spec/solidity/contracts/executable/AxelarExecutable.sol";


/// @title Axelar Memo Contract
/// @dev This contract provides functionalities to send and receive a memo message to Solana using Axelar Gateway
contract AxelarMemo is AxelarExecutable {

    /// @dev The number of messages received
    uint256 public MESSAGES_RECEIVED;

    /// @dev Event emitted when a memo message is received
    /// @param memoMessage The memo message received
    event ReceivedMemo(string memoMessage);

    constructor(address gateway_) AxelarExecutable(gateway_) {
        MESSAGES_RECEIVED = 0;
    }

    /// @dev Sends a memo message to Solana using the Axelar Gateway
    /// @param solanaDestinationProgram The destination Solana program to send the memo to.
    ///        This is supposed to be the base58 encoded representation of [u8; 32] bytes of the program ID.
    /// @param solanaChain The Solana chain identifier to send the memo to.
    ///        This is the unique chain ID registered within the Axelar Network.
    /// @param memoMessage The memo message to send
    /// @param accounts The accounts that will be used in the Solana transaction.
    ///        Because Accounts in solana is part of the public interface, they need to be supplied here.
    function sendToSolana(
        string calldata solanaDestinationProgram,
        bytes calldata solanaChain,
        bytes calldata memoMessage,
        SolanaAccountRepr[] calldata accounts
    ) external {
        SolanaGatewayPayload memory payload =
            SolanaGatewayPayload({executePayload: abi.encodePacked(memoMessage), accounts: accounts});

        bytes memory encodedPayload = payload.encode();
        gateway.callContract(string(solanaChain), solanaDestinationProgram, encodedPayload);
    }

    /// @dev Receives a memo message from Solana
    function _execute(string calldata, string calldata, bytes calldata payload) internal override {
        MESSAGES_RECEIVED += 1;

        string memory converted = string(payload);
        emit ReceivedMemo(converted);
    }
}
