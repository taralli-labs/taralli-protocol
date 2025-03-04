// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

import "forge-std/Script.sol";
import "forge-std/Test.sol";
import "src/UniversalBombetta.sol";
import "src/UniversalPorchetta.sol";
import "src/interfaces/IPermit2.sol";

contract Deploy is Script, Test {
    // default is Holesky testnet
    string RPC_URL = vm.envString("ETH_HOLESKY_RPC_URL");
    // Canonical Permit2 contract
    IPermit2 public immutable PERMIT2 = IPermit2(0x000000000022D473030F116dDEE9F6B43aC78BA3);

    function run() public {
        uint256 deployerPrivateKey = vm.envUint("LOCAL_PRIVATE_KEY");
        vm.createSelectFork(RPC_URL);
        vm.startBroadcast(deployerPrivateKey);

        // deploy bombetta market
        UniversalBombetta universalBombetta = new UniversalBombetta(PERMIT2);
        emit log_named_address("Universal Bombetta Address", address(universalBombetta));

        // deploy porchetta market
        UniversalPorchetta universalPorchetta = new UniversalPorchetta(PERMIT2);
        emit log_named_address("Universal Porchetta Address", address(universalPorchetta));

        vm.stopBroadcast();

        // Start with an empty JSON object
        string memory deploymentAddresses = "deployments";
        // Serialize addresses to the JSON object
        vm.serializeAddress(deploymentAddresses, "universal_bombetta", address(universalBombetta));
        vm.serializeAddress(deploymentAddresses, "universal_porchetta", address(universalPorchetta));

        string memory jsonOutput = vm.serializeString(deploymentAddresses, "object", "object");

        // Write deployment addresses to JSON file
        vm.writeJson(jsonOutput, "./deployments.json");
    }
}
