// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

import "../BaseTest.sol";
import "src/libraries/BombettaTypes.sol";
import "src/libraries/BombettaErrors.sol";

// risc0 ethereum
import "risc0/groth16/RiscZeroGroth16Verifier.sol";
import "risc0/test/RiscZeroCheats.sol";

contract Risc0UniversalBombettaTest is BaseTest {
    RiscZeroGroth16Verifier public risc0Verifier;

    function setUp() external {
        _setUp();

        // deploy risc0 verifier
        // Read verifier configuration data from proof.json
        string memory proofDataJson = vm.readFile("./test-proof-data/risc0/even-number-proof.json");
        // read control root and bn254 control ID
        bytes32 controlRoot = vm.parseJsonBytes32(proofDataJson, "$.control_root");
        bytes32 bn254ControlId = vm.parseJsonBytes32(proofDataJson, "$.bn254_control_id");

        risc0Verifier = new RiscZeroGroth16Verifier(controlRoot, bn254ControlId);
        //emit log_named_bytes32("risc0Verifier.SELECTOR()", risc0Verifier.SELECTOR());
    }

    function testRisc0UniversalBombettaProofRequest() public {
        // Read proof data from proof.json
        string memory proofDataJson = vm.readFile("./test-proof-data/risc0/even-number-proof.json");
        bytes32 imageId = vm.parseJsonBytes32(proofDataJson, "$.image_id");

        // requester computes guest program inputs commitment
        uint256 proofInput = 1304;
        bytes32 proofInputHash = sha256(abi.encode(proofInput));
        bytes32 publicInputsCommitment = sha256(abi.encode(imageId, proofInputHash));
        emit log_named_bytes32("publicInputsCommitment", publicInputsCommitment);

        /// Set up the proof request data
        // Metadata.extraData
        UniversalBombetta.VerifierDetails memory verifierDetails = UniversalBombetta.VerifierDetails({
            verifier: address(risc0Verifier),
            selector: risc0Verifier.verify.selector,
            isShaCommitment: true,
            publicInputsOffset: 32,
            publicInputsLength: 64,
            hasPartialCommitmentResultCheck: false,
            submittedPartialCommitmentResultOffset: 0,
            submittedPartialCommitmentResultLength: 0,
            predeterminedPartialCommitment: bytes32(0)
        });

        // ProofRequest
        ProofRequest memory request = ProofRequest({
            signer: alice,
            market: address(universalBombetta),
            nonce: 0,
            token: address(testToken),
            maxRewardAmount: 1000 ether, // 1000 tokens
            minRewardAmount: 0,
            minimumStake: 1 ether,
            startAuctionTimestamp: uint64(block.timestamp),
            endAuctionTimestamp: uint64(block.timestamp + 1000),
            provingTime: 1 days,
            publicInputsCommitment: publicInputsCommitment,
            extraData: abi.encode(verifierDetails)
        });

        bytes memory sig = _getBombettaSignature(address(universalBombetta), request, ALICE_PK);

        vm.warp(block.timestamp + 10);

        //_logAssetBalances("pre bid state", address(universalBombetta), alice, bob);

        // Submit the bid as the prover (Bob)
        vm.startPrank(bob);
        (uint256 rewardAmount,) = universalBombetta.bid{value: 1 ether}(request, sig);
        vm.stopPrank();

        // Assert the transfer of the eth stake and the erc20 token reward worked and are now in the bombetta
        assertEq(address(universalBombetta).balance, 1 ether);
        assertEq(testToken.balanceOf(address(universalBombetta)), rewardAmount);

        // forward 100 seconds
        vm.warp(block.timestamp + 100);

        // get risc0 proof submission
        bytes memory opaqueSubmission = _getRisc0ProofSubmission();

        uint256 prestateBobTokenBalance = testToken.balanceOf(bob);

        //_logAssetBalances("post bid & pre resolve state", address(universalBombetta), alice, bob);

        // get request ID
        bytes32 requestId = universalBombetta.computeRequestId(request, sig);

        // bob (solver) resolves proof request
        vm.startPrank(bob);
        universalBombetta.resolve(requestId, opaqueSubmission, bytes32(0));
        vm.stopPrank();

        //_logAssetBalances("post resolve state", address(universalBombetta), alice, bob);

        // assert funds transferred out of bombetta contract
        assertEq(address(universalBombetta).balance, 0);
        assertEq(testToken.balanceOf(address(universalBombetta)), 0);
        // assert funds correctly transferred to solver/prover for resolving the proof request
        assertEq(bob.balance, 10 ether);
        assertEq(testToken.balanceOf(bob) - prestateBobTokenBalance, rewardAmount);
    }

    /////////////////////////////////// HELPERS /////////////////////////////////////////

    function _getRisc0ProofSubmission() public returns (bytes memory) {
        // Read proof data from proof.json
        string memory proofDataJson = vm.readFile("./test-proof-data/risc0/even-number-proof.json");

        // read journal digest
        bytes32 journalDigest = vm.parseJsonBytes32(proofDataJson, "$.journal_digest");
        // read image id
        bytes32 imageId = vm.parseJsonBytes32(proofDataJson, "$.image_id");
        // read seal
        bytes memory seal = vm.parseJsonBytes(proofDataJson, "$.seal");

        // Encode the data in the format expected by the verify function
        bytes memory opaqueSubmission = abi.encode(seal, imageId, journalDigest);
        emit log_named_bytes("TEST: opaqueSubmission", opaqueSubmission);

        return opaqueSubmission;
    }
}
