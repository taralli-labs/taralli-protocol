pragma solidity ^0.8.0;

import "permit2/interfaces/ISignatureTransfer.sol";

interface IPermit2 is ISignatureTransfer {
    // permit2's signature transfer & eip712 interface
    function DOMAIN_SEPARATOR() external view returns (bytes32);
}
