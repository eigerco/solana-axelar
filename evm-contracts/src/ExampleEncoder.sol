// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.19;

import {AbiSolanaGatewayPayload, SolanaGatewayPayload, SolanaAccountRepr} from "./SolanaGatewayPayload.sol";


/**
 * @title Example Solana Gateway Encoder
 * @dev This contract provides functionalities to encode and decode SolanaGatewayPayload structures
 */
contract ExampleSolanaGatewayEncoder {
    /**
     * @dev Encodes a SolanaGatewayPayload structure into a bytes format.
     * @param payload The SolanaGatewayPayload to encode.
     * @return The encoded payload as bytes.
     */
    function encode(SolanaGatewayPayload calldata payload) public pure returns (bytes memory) {
        return payload.encode();
    }

    /**
     * @dev Decodes a bytes object back into a SolanaGatewayPayload structure.
     * @param data The bytes object to decode.
     * @return The decoded SolanaGatewayPayload structure.
     */
    function decode(bytes calldata data) public pure returns (SolanaGatewayPayload memory) {
        return AbiSolanaGatewayPayload.decode(data);
    }
}
