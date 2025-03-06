// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

import "../BaseTest.sol";
import "src/libraries/BombettaTypes.sol";
import "permit2/interfaces/ISignatureTransfer.sol";

contract UniversalBombettaTest is BaseTest {
    function setUp() external {
        _setUp();
    }

    function testUniversalBombettaProofRequest() public {
        /// Set up the proof request data
        // extraData
        UniversalBombetta.VerifierDetails memory verifierDetails = UniversalBombetta.VerifierDetails({
            verifier: address(verifierG16),
            selector: verifierG16.verifyProof.selector,
            isShaCommitment: false,
            inputsOffset: 256,
            inputsLength: 32,
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
            rewardToken: address(testToken),
            maxRewardAmount: 1000 ether, // 1000 tokens
            minRewardAmount: 0,
            minimumStake: 1 ether,
            startAuctionTimestamp: uint64(block.timestamp),
            endAuctionTimestamp: uint64(block.timestamp + 1000),
            provingTime: 1 days,
            inputsCommitment: keccak256(abi.encode(33)),
            extraData: abi.encode(verifierDetails)
        });

        bytes memory sig = _getBombettaSignature(address(universalBombetta), request, ALICE_PK);

        emit log_named_bytes("signature", sig);

        vm.warp(block.timestamp + 10);

        // Submit the bid as the prover (Bob)
        vm.startPrank(bob);
        (, uint256 rewardAmount,) = universalBombetta.bid{value: 1 ether}(request, sig);
        vm.stopPrank();

        // Assert the transfer of the eth stake and the erc20 token reward worked and are now in the bombetta
        assertEq(address(universalBombetta).balance, 1 ether);
        assertEq(testToken.balanceOf(address(universalBombetta)), rewardAmount);

        // get request ID
        bytes32 requestId = universalBombetta.computeRequestId(request, sig);

        // forward 100 seconds
        vm.warp(block.timestamp + 100);

        // encode opaque submission
        bytes memory opaqueSubmission = _getGroth16ProofSubmission();

        uint256 prestateBobTokenBalance = testToken.balanceOf(bob);

        // bob (solver) resolves proof request
        vm.startPrank(bob);
        universalBombetta.resolve(requestId, opaqueSubmission, bytes32(0));
        vm.stopPrank();

        // assert funds transferred out of bombetta contract
        assertEq(address(universalBombetta).balance, 0);
        assertEq(testToken.balanceOf(address(universalBombetta)), 0);
        // assert funds correctly transferred to solver/prover for resolving the proof request
        assertEq(bob.balance, 10 ether);
        assertEq(testToken.balanceOf(bob) - prestateBobTokenBalance, rewardAmount);
    }

    function testUniversalBidErrors() public {
        ProofRequest memory mockRequest = ProofRequest({
            signer: alice,
            provingTime: 1 days,
            nonce: 0,
            rewardToken: address(testToken),
            maxRewardAmount: 0,
            minRewardAmount: 0,
            market: address(universalBombetta),
            startAuctionTimestamp: uint64(block.timestamp + 1),
            minimumStake: 2,
            endAuctionTimestamp: uint64(block.timestamp + 1),
            inputsCommitment: keccak256(abi.encode(1)),
            extraData: bytes("")
        });

        bytes memory mockSig = "";

        vm.startPrank(bob);

        // start timestamp for proof request auction has not been reached yet, InvalidRequest()
        vm.expectRevert(InvalidRequest.selector);
        universalBombetta.bid{value: 1}(mockRequest, mockSig);

        // change start timestamp to valid to test other error cases
        mockRequest.startAuctionTimestamp = uint64(block.timestamp);

        // minimum stake requirement not met
        vm.expectRevert(InvalidRequest.selector);
        universalBombetta.bid{value: 1}(mockRequest, mockSig);

        // request deadline now expired
        vm.warp(block.timestamp + 2);

        // request deadline has passed
        vm.expectRevert(InvalidRequest.selector);
        universalBombetta.bid{value: 1 ether}(mockRequest, mockSig);

        ProofRequest memory request = ProofRequest({
            signer: alice,
            provingTime: 1 days,
            nonce: 0,
            rewardToken: address(testToken),
            maxRewardAmount: 1000 ether, // 1000 tokens
            minRewardAmount: 0,
            market: address(universalBombetta),
            startAuctionTimestamp: uint64(block.timestamp),
            minimumStake: 1 ether,
            endAuctionTimestamp: uint64(block.timestamp + 1000),
            inputsCommitment: keccak256(abi.encode(33)),
            extraData: bytes("")
        });

        bytes memory sig = _getBombettaSignature(address(universalBombetta), request, ALICE_PK);

        vm.warp(block.timestamp + 10);

        // Submit the 1st valid bid
        universalBombetta.bid{value: 1 ether}(request, sig);

        // attempt 2nd bid within deadline, should fail
        vm.expectRevert(AuctionEnded.selector);
        universalBombetta.bid{value: 1 ether}(request, sig);

        vm.stopPrank();
    }

    function testUniversalResolutionAlternativeResolverBeforeDeadlineError() public {
        /// Set up the proof request data
        // Metadata.extraData
        UniversalBombetta.VerifierDetails memory verifierDetails = UniversalBombetta.VerifierDetails({
            verifier: address(verifierG16),
            selector: verifierG16.verifyProof.selector,
            isShaCommitment: false,
            inputsOffset: 256,
            inputsLength: 32,
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
            rewardToken: address(testToken),
            maxRewardAmount: 1000 ether, // 1000 tokens
            minRewardAmount: 0,
            minimumStake: 1 ether,
            startAuctionTimestamp: uint64(block.timestamp),
            endAuctionTimestamp: uint64(block.timestamp + 1000),
            provingTime: 1 days,
            inputsCommitment: keccak256(abi.encode(33)),
            extraData: abi.encode(verifierDetails)
        });

        bytes memory sig = _getBombettaSignature(address(universalBombetta), request, ALICE_PK);

        vm.warp(block.timestamp + 10);

        // Submit the bid as the prover (Bob)
        vm.startPrank(bob);
        (, uint256 rewardAmount,) = universalBombetta.bid{value: 1 ether}(request, sig);
        vm.stopPrank();

        // Assert the transfer of the eth stake and the erc20 token reward worked and are now in the bombetta
        assertEq(address(universalBombetta).balance, 1 ether);
        assertEq(testToken.balanceOf(address(universalBombetta)), rewardAmount);

        // get request ID
        bytes32 requestId = universalBombetta.computeRequestId(request, sig);

        // forward 100 seconds
        vm.warp(block.timestamp + 100);

        // encode opaque submission
        bytes memory opaqueSubmission = _getGroth16ProofSubmission();

        // alternative caller that didnt bid on the request attempts to resolve
        // even with the correct opaqueSubmission/proof this should revert
        // because 0xbeef does not have the right to resolve this request
        // until the resolutionDeadline has passed for bob who bid for the rights
        // to make the proof for this request
        vm.startPrank(address(0xbeef));
        vm.expectRevert(InvalidResolver.selector);
        universalBombetta.resolve(requestId, opaqueSubmission, bytes32(0));

        // check the same error case with an empty opaqueSubmission
        vm.expectRevert(InvalidResolver.selector);
        universalBombetta.resolve(requestId, "", bytes32(0));
        vm.stopPrank();
    }

    function testUniversalResolutionPassDeadline() public {
        ProofRequest memory request = ProofRequest({
            signer: alice,
            provingTime: 1 days,
            nonce: 0,
            rewardToken: address(testToken),
            maxRewardAmount: 1000 ether, // 1000 tokens
            minRewardAmount: 0,
            market: address(universalBombetta),
            startAuctionTimestamp: uint64(block.timestamp),
            minimumStake: 1 ether,
            endAuctionTimestamp: uint64(block.timestamp + 1000),
            inputsCommitment: keccak256(abi.encode(33)),
            extraData: bytes("")
        });

        bytes memory sig = _getBombettaSignature(address(universalBombetta), request, ALICE_PK);

        vm.warp(block.timestamp + 10);

        uint256 aliceBeforeBidTokenBalance = testToken.balanceOf(alice);

        // Submit the bid as the prover (Bob)
        vm.startPrank(bob);
        universalBombetta.bid{value: 1 ether}(request, sig);
        vm.stopPrank();

        // forward 1 day + 1 second
        vm.warp(block.timestamp + 1 days + 1);

        // encode opaque submission
        ModableTestSubmission memory submission = _getModableGroth16ProofSubmission();
        bytes memory opaqueSubmission = abi.encode(submission);

        uint256 aliceEthBalBeforeResolve = alice.balance;
        uint256 prestateBobTokenBalance = testToken.balanceOf(bob);

        // get request ID
        bytes32 requestId = universalBombetta.computeRequestId(request, sig);

        // bob (solver) resolves proof request
        vm.startPrank(bob);
        universalBombetta.resolve(requestId, opaqueSubmission, bytes32(0));
        vm.stopPrank();

        // assert slash behavior from pass deadline submission slashes
        // assert funds transferred out of bombetta contract
        assertEq(address(universalBombetta).balance, 0);
        assertEq(testToken.balanceOf(address(universalBombetta)), 0);
        // assert funds are not transferred to solver/prover
        assertEq(bob.balance, 9 ether);
        assertEq(testToken.balanceOf(bob) - prestateBobTokenBalance, 0);
        // assert requester receives eth stake from slash and their tokens back
        assertEq(alice.balance, aliceEthBalBeforeResolve + 1 ether);
        assertEq(testToken.balanceOf(alice), aliceBeforeBidTokenBalance);
    }

    function testUniversalResolutionPassDeadlineAlternativeResolver() public {
        // Set up the proof request data
        ProofRequest memory request = ProofRequest({
            signer: alice,
            provingTime: 1 days,
            nonce: 0,
            rewardToken: address(testToken),
            maxRewardAmount: 1000 ether, // 1000 tokens
            minRewardAmount: 0,
            market: address(universalBombetta),
            startAuctionTimestamp: uint64(block.timestamp),
            minimumStake: 1 ether,
            endAuctionTimestamp: uint64(block.timestamp + 1000),
            inputsCommitment: keccak256(abi.encode(33)),
            extraData: bytes("")
        });

        bytes memory sig = _getBombettaSignature(address(universalBombetta), request, ALICE_PK);

        vm.warp(block.timestamp + 10);

        uint256 aliceBeforeBidTokenBalance = testToken.balanceOf(alice);

        // Submit the bid as the prover (Bob)
        vm.startPrank(bob);
        universalBombetta.bid{value: 1 ether}(request, sig);
        vm.stopPrank();

        // forward 1 day + 1 second
        vm.warp(block.timestamp + 1 days + 1);

        uint256 aliceEthBalBeforeResolve = alice.balance;
        uint256 prestateBobTokenBalance = testToken.balanceOf(bob);

        // get request ID
        bytes32 requestId = universalBombetta.computeRequestId(request, sig);

        // alice resolves proof request instead of bob as deadline has passed
        vm.startPrank(alice);
        universalBombetta.resolve(requestId, bytes(""), bytes32(0));
        vm.stopPrank();

        // assert slash behavior from pass deadline submission slashes
        // assert funds transferred out of bombetta contract
        assertEq(address(universalBombetta).balance, 0);
        assertEq(testToken.balanceOf(address(universalBombetta)), 0);
        // assert funds are not transferred to solver/prover
        assertEq(bob.balance, 9 ether);
        assertEq(testToken.balanceOf(bob) - prestateBobTokenBalance, 0);
        // assert requester receives eth stake from slash and their tokens back
        assertEq(alice.balance, aliceEthBalBeforeResolve + 1 ether);
        assertEq(testToken.balanceOf(alice), aliceBeforeBidTokenBalance);
    }

    function testComputeRequestId() public {
        // make request
        ProofRequest memory request = ProofRequest({
            signer: alice,
            provingTime: 1 days,
            nonce: 0,
            rewardToken: address(testToken),
            maxRewardAmount: 1000 ether, // 1000 tokens
            minRewardAmount: 0,
            market: address(universalBombetta),
            startAuctionTimestamp: uint64(block.timestamp),
            minimumStake: 1 ether,
            endAuctionTimestamp: uint64(block.timestamp + 1000),
            inputsCommitment: keccak256(abi.encode(33)),
            extraData: bytes("")
        });

        //bytes memory sig = "";
        bytes memory sig = abi.encodePacked(
            bytes32(0x840cfc572845f5786e702984c2a582528cad4b49b2a10b9db1be7fca90058565), // r
            bytes32(0x25e7109ceb98168d95b09b18bbf6b685130e0562f233877d492b94eee0c5b6d1), // s
            uint8(27) // v in Ethereum
        );

        _logProofRequest("solidity mock request:", request, sig);

        bytes32 requestId = universalBombetta.computeRequestId(request, sig);
        emit log_named_bytes32("solidity request id", requestId);
        emit log_named_bytes32(
            "local rs request id", bytes32(0x0fa6eb199b3fb79f2c946a00fc4da80427a7ce424a1cc199afa79dc7e312d291)
        );
    }

    function testComputeRequestWitness() public {
        // make request
        ProofRequest memory request = ProofRequest({
            signer: alice,
            provingTime: 1 days,
            nonce: 0,
            rewardToken: address(testToken),
            maxRewardAmount: 1000 ether, // 1000 tokens
            minRewardAmount: 0,
            market: address(universalBombetta),
            startAuctionTimestamp: uint64(block.timestamp),
            minimumStake: 1 ether,
            endAuctionTimestamp: uint64(block.timestamp + 1000),
            inputsCommitment: keccak256(abi.encode(33)),
            extraData: bytes("")
        });

        bytes32 witness = universalBombetta.computeWitnessHash(request);
        emit log_named_bytes32("witness hash:", witness);
    }

    function testComputeRequestSigning() public {
        // make request
        ProofRequest memory request = ProofRequest({
            signer: address(0x0000000000000000000000000000000000000001),
            market: address(0x0000000000000000000000000000000000000001),
            nonce: 0,
            rewardToken: address(0x0000000000000000000000000000000000000001),
            maxRewardAmount: 0,
            minRewardAmount: 0,
            minimumStake: 0,
            startAuctionTimestamp: uint64(0),
            endAuctionTimestamp: uint64(0),
            provingTime: 0,
            inputsCommitment: bytes32(0),
            extraData: bytes("")
        });

        // mock witness
        bytes32 witness = universalBombetta.computeWitnessHash(request);
        emit log_named_bytes32("witness hash", witness);

        // build permit
        ISignatureTransfer.TokenPermissions memory tokenPermissions =
            ISignatureTransfer.TokenPermissions({token: request.rewardToken, amount: request.maxRewardAmount});
        ISignatureTransfer.PermitTransferFrom memory permit = ISignatureTransfer.PermitTransferFrom({
            permitted: tokenPermissions,
            nonce: request.nonce,
            deadline: request.endAuctionTimestamp
        });

        bytes32 digest = universalBombetta.computePermitDigest(permit, witness);
        emit log_named_bytes32("permit digest:", digest);
    }

    /////////////////////////////////// HELPERS /////////////////////////////////////////
}
