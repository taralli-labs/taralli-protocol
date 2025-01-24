// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

import "../BaseTest.sol";
import "src/libraries/PorchettaTypes.sol";

contract UniversalPorchettaTest is BaseTest {
    function setUp() external {
        _setUp();
    }

    function testUniversalPorchettaProofOffer() public {
        /// Set up the prover intent data
        // extraData
        UniversalPorchetta.VerifierDetails memory verifierDetails = UniversalPorchetta.VerifierDetails({
            verifier: address(verifierG16),
            selector: verifierG16.verifyProof.selector,
            isShaCommitment: false,
            inputsOffset: 256,
            inputsLength: 32
        });

        // ProofOffer
        ProofOffer memory offer = ProofOffer({
            signer: bob,
            market: address(universalPorchetta),
            nonce: 0,
            rewardToken: address(testToken),
            rewardAmount: 1000 ether, // 1000 tokens
            stakeToken: address(testToken),
            stakeAmount: 1000 ether,
            startAuctionTimestamp: uint64(block.timestamp),
            endAuctionTimestamp: uint64(block.timestamp + 1000),
            provingTime: 1 days,
            inputsCommitment: keccak256(abi.encode(33)),
            extraData: abi.encode(verifierDetails)
        });

        bytes memory sig = _getPorchettaSignature(address(universalPorchetta), offer, BOB_PK);

        vm.warp(block.timestamp + 10);

        // alice approves the market contract
        vm.prank(alice);
        testToken.approve(address(universalPorchetta), type(uint256).max);

        // Submit the bid as the requester (alice)
        vm.startPrank(alice);
        (, uint256 rewardAmount,) = universalPorchetta.bid(offer, sig);
        vm.stopPrank();

        // Assert the transfer of the stake and token reward worked and are now in the porchetta (reward amount + token stake)
        assertEq(testToken.balanceOf(address(universalPorchetta)), rewardAmount + 1000 ether);
        assertEq(testToken.balanceOf(bob), 100000 ether - 1000 ether); // both bob & alice give 1000 tokens each to the market
        assertEq(testToken.balanceOf(alice), 100000 ether - 1000 ether);

        // forward 100 seconds
        vm.warp(block.timestamp + 100);

        // encode opaque submission
        bytes32 offerId = universalPorchetta.computeOfferId(offer);
        bytes memory opaqueSubmission = _getGroth16ProofSubmission();
        bytes32 submittedPartialCommitment = bytes32(0);

        uint256 prestateBobTokenBalance = testToken.balanceOf(bob);

        // bob (solver) resolves prover intent
        vm.startPrank(bob);
        universalPorchetta.resolve(offerId, opaqueSubmission);
        vm.stopPrank();

        // assert funds transferred out of porchetta contract
        assertEq(testToken.balanceOf(address(universalPorchetta)), 0);
        // assert funds correctly transferred to solver/prover for resolving the prover intent
        assertEq(testToken.balanceOf(bob) - prestateBobTokenBalance, rewardAmount + offer.stakeAmount);
    }
}
