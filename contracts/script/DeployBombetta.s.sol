// SPDX-License-Identifier: UNLICENSED
pragma solidity 0.8.23;

import "forge-std/Script.sol";
import "forge-std/Test.sol";
import "src/UniversalBombetta.sol";
import "src/verifiers/SimpleGroth16Verifier.sol";
import "risc0/groth16/RiscZeroGroth16Verifier.sol";
import "src/interfaces/IPermit2.sol";
import "test/mocks/ERC20Mock.sol";

contract DeployBombetta is Script, Test {
    // local anvil instance must fork from ethereum main-net for permit2
    string RPC_URL = vm.envString("ETH_HOLESKY_RPC_URL");
    // Canonical Permit2 contract
    IPermit2 public immutable PERMIT2 = IPermit2(0x000000000022D473030F116dDEE9F6B43aC78BA3);

    function run() public {
        uint256 deployerPrivateKey = vm.envUint("LOCAL_PRIVATE_KEY");
        vm.createSelectFork(RPC_URL);
        vm.startBroadcast(deployerPrivateKey);

        // deploy bombetta
        UniversalBombetta universalBombetta = new UniversalBombetta(PERMIT2);
        emit log_named_address("Universal Bombetta Address", address(universalBombetta));

        vm.stopBroadcast();
    }
}
