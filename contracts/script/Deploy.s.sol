// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.23;

import "forge-std/Script.sol";
import "forge-std/Test.sol";
import "src/UniversalBombetta.sol";
import "src/UniversalPorchetta.sol";
import "src/verifiers/SimpleGroth16Verifier.sol";
import "src/verifiers/GnarkVerifier.sol";
import "risc0/groth16/RiscZeroGroth16Verifier.sol";
import "src/interfaces/IPermit2.sol";
import "test/mocks/ERC20Mock.sol";

contract Deploy is Script, Test {
    string RPC_URL = vm.envString("ETH_SEPOLIA_RPC_URL");
    // Canonical Permit2 contract
    IPermit2 public immutable PERMIT2 = IPermit2(0x000000000022D473030F116dDEE9F6B43aC78BA3);

    function run() public {
        uint256 deployerPrivateKey = vm.envUint("REQUESTER_PRIVATE_KEY");
        vm.createSelectFork(RPC_URL);
        vm.startBroadcast(deployerPrivateKey);

        // deloy groth16 verifier for simple circuit
        SimpleGroth16Verifier groth16Verifier = new SimpleGroth16Verifier();
        // deploy risc0 verifier for risc0 proof requests with corresponding control root & id
        string memory proofDataJson = vm.readFile("./test-proof-data/risc0/even-number-proof.json");
        bytes32 controlRoot = vm.parseJsonBytes32(proofDataJson, "$.control_root");
        bytes32 bn254ControlId = vm.parseJsonBytes32(proofDataJson, "$.bn254_control_id");
        RiscZeroGroth16Verifier risc0Verifier = new RiscZeroGroth16Verifier(controlRoot, bn254ControlId);

        // deploy bombetta
        UniversalBombetta universalBombetta = new UniversalBombetta(PERMIT2);
        emit log_named_address("Universal Bombetta Address", address(universalBombetta));

        // deploy porchetta market
        UniversalPorchetta universalPorchetta = new UniversalPorchetta(PERMIT2);
        emit log_named_address("Universal Porchetta Address", address(universalPorchetta));

        // deploy test token
        ERC20Mock testToken = new ERC20Mock("Test Token", "TEST", 18);
        emit log_named_address("Reward Token Address", address(testToken));

        vm.stopBroadcast();

        // Start with an empty JSON object
        string memory deploymentAddresses = "deployments";
        // Serialize addresses to the JSON object
        vm.serializeAddress(deploymentAddresses, "universal_bombetta", address(universalBombetta));
        vm.serializeAddress(deploymentAddresses, "universal_porchetta", address(universalPorchetta));
        vm.serializeAddress(deploymentAddresses, "test_token", address(testToken));
        vm.serializeAddress(deploymentAddresses, "risc0_verifier", address(risc0Verifier));
        vm.serializeAddress(deploymentAddresses, "groth16_verifier", address(groth16Verifier));
        vm.serializeAddress(deploymentAddresses, "groth16_verifier", address(groth16Verifier));

        string memory jsonOutput = vm.serializeString(deploymentAddresses, "object", "object");

        // Write deployment addresses to JSON file
        vm.writeJson(jsonOutput, "./deployments.json");
    }
}
