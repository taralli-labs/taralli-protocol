// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

import "../../BaseTest.sol";
import "forge-std/Test.sol";
import "src/UniversalBombetta.sol";
import "src/libraries/BombettaTypes.sol";

contract BombettaActor is Test {
    struct ProofRequestData {
        ProofRequest request;
        bytes signature;
        bytes32 requestId;
    }

    ProofRequestData[] public allProofRequests;
    mapping(uint256 => bool) public hasBeenBidOn;
    mapping(bytes32 => bool) public hasBeenResolved;
    bytes32[] public activeRequestIds;
    mapping(bytes32 => ProofRequestData) public activeProofRequestData;
    mapping(bytes32 => uint256) public bidTimestamps;

    uint256 public bidCount;
    uint256 public resolveCount;
    uint256 public currentNonce;

    // state needed from base test environment
    address internal alice; // proof requester
    uint256 internal ALICE_PK; // for request signatures
    address internal bob; // proof provider
    BaseTest internal baseTest; // utils for computing signature
    ERC20Mock internal testToken; // test reward token
    UniversalBombetta internal universalBombetta; // bombetta market
    SimpleGroth16Verifier internal verifierG16; // verifier contract

    constructor(
        address _alice,
        uint256 _ALICE_PK,
        address _bob,
        address _baseTest,
        address _testToken,
        address _universalBombetta,
        address _verifierG16
    ) {
        alice = _alice;
        ALICE_PK = _ALICE_PK;
        bob = _bob;
        baseTest = BaseTest(_baseTest);
        testToken = ERC20Mock(_testToken);
        universalBombetta = UniversalBombetta(_universalBombetta);
        verifierG16 = SimpleGroth16Verifier(_verifierG16);
        // make initial proof request
        (ProofRequest memory initialRequest, bytes memory sig) = _initializeFirstOutstandingProofRequest();
        bytes32 initRequestId = universalBombetta.computeRequestId(initialRequest, sig);
        ProofRequestData memory requestData =
            ProofRequestData({request: initialRequest, signature: sig, requestId: initRequestId});
        allProofRequests.push(requestData);
        hasBeenBidOn[0] = false;
        currentNonce = 1;
    }

    /// @notice Alice (proof requester) creates and signs a new outstanding proof request
    function createAndSignProofRequest(
        uint256 fuzzedProvingTime,
        uint256 fuzzedTokenAmount,
        uint256 fuzzedMinReward,
        uint64 fuzzedStartTimestamp,
        uint128 fuzzedMinimumStake,
        uint256 fuzzedDeadline
    ) public returns (ProofRequest memory, bytes memory) {
        emit log_string("ACTOR: createAndSignProofRequest start");
        // bound stuff
        uint256 bobAvailableBalance =
            bob.balance - _bobTotalPotentialEthObligationFromBiddingOnOutstandingProofRequests();
        uint256 aliceAvailableBalance =
            testToken.balanceOf(alice) - _aliceTotalPotentialTokenObligationFromOutstandingProofRequests();
        uint32 provingTime = uint32(bound(fuzzedProvingTime, 3, 3 days));
        uint256 tokenAmount = bound(fuzzedTokenAmount, 0, aliceAvailableBalance);
        uint256 minReward = bound(fuzzedMinReward, 0, tokenAmount);
        uint64 startAuctionTimestamp = uint64(bound(fuzzedStartTimestamp, block.timestamp, block.timestamp + 30000));
        uint128 minimumStake = uint128(bound(fuzzedMinimumStake, 0, bobAvailableBalance));
        uint64 endAuctionTimestamp =
            uint64(bound(fuzzedDeadline, startAuctionTimestamp + 3, startAuctionTimestamp + 300));

        currentNonce++; // increment signature nonce

        // extraData
        UniversalBombetta.VerifierDetails memory verifierDetails = UniversalBombetta.VerifierDetails({
            verifier: address(verifierG16),
            selector: verifierG16.verifyProof.selector,
            isShaCommitment: false,
            publicInputsOffset: 256,
            publicInputsLength: 32,
            hasPartialCommitmentResultCheck: false,
            submittedPartialCommitmentResultOffset: 0,
            submittedPartialCommitmentResultLength: 0,
            predeterminedPartialCommitment: bytes32(0)
        });

        ProofRequest memory request = ProofRequest({
            signer: alice,
            provingTime: provingTime,
            nonce: currentNonce,
            token: address(testToken),
            maxRewardAmount: tokenAmount,
            minRewardAmount: minReward,
            market: address(universalBombetta),
            startAuctionTimestamp: startAuctionTimestamp,
            minimumStake: minimumStake,
            endAuctionTimestamp: endAuctionTimestamp,
            publicInputsCommitment: keccak256(abi.encode(33)),
            extraData: abi.encode(verifierDetails)
        });

        // compute alice's signature
        bytes memory sig = baseTest._getBombettaSignature(address(universalBombetta), request, ALICE_PK);

        baseTest._logProofRequest(
            "---------- ACTOR: createAndSignProofRequest() CREATED PROOF REQUEST -------------", request, sig
        );

        bytes32 requestId = universalBombetta.computeRequestId(request, sig);

        ProofRequestData memory requestData = ProofRequestData({request: request, signature: sig, requestId: requestId});

        allProofRequests.push(requestData);
        hasBeenBidOn[allProofRequests.length - 1] = false;
        return (request, sig);
    }

    /// @notice bob (proof provider) bids on a randomly selected outstanding proof request
    function bid(uint256 fuzzedIndex, uint256 fuzzedWaitTime) external {
        emit log_string("ACTOR: bid start");
        uint256 unbidRequestCount = fetchUnbidRequestsCount();
        if (unbidRequestCount == 0) {
            // If no unbid requests, create a new one and bid on it
            createAndBid(
                fuzzedIndex, // reuse fuzzedIndex for rand uints
                fuzzedIndex,
                fuzzedIndex,
                uint64(fuzzedIndex),
                uint128(fuzzedIndex),
                fuzzedIndex,
                fuzzedIndex
            );
            return;
        }

        uint256 targetIndex = bound(fuzzedIndex, 0, unbidRequestCount - 1);
        uint256 actualIndex;
        uint256 currentUnbidCount = 0;

        for (uint256 i = 0; i < allProofRequests.length; i++) {
            if (!hasBeenBidOn[i]) {
                if (currentUnbidCount == targetIndex) {
                    actualIndex = i;
                    break;
                }
                currentUnbidCount++;
            }
        }

        ProofRequestData storage requestData = allProofRequests[actualIndex];

        baseTest._logProofRequest(
            "---------- ACTOR: bid() SELECTED PROOF REQUEST -------------", requestData.request, requestData.signature
        );
        baseTest._logAssetBalances(
            "---------- ACTOR: bid() pre state -------------", address(universalBombetta), alice, bob
        );

        // Wait a variable amount of time between now and the deadline of the auction to bid
        uint256 bidTimestamp =
            bound(fuzzedWaitTime, requestData.request.startAuctionTimestamp, requestData.request.endAuctionTimestamp);
        vm.warp(bidTimestamp);

        // call the bid function as the proof provider/solver
        vm.prank(bob);
        universalBombetta.bid{value: requestData.request.minimumStake}(requestData.request, requestData.signature);

        baseTest._logAssetBalances(
            "---------- ACTOR: bid() post state -------------", address(universalBombetta), alice, bob
        );

        bytes32 requestId = universalBombetta.computeRequestId(requestData.request, requestData.signature);
        activeRequestIds.push(requestId);
        activeProofRequestData[requestId] = requestData;
        bidTimestamps[requestId] = bidTimestamp;

        hasBeenBidOn[actualIndex] = true;
        bidCount++;
    }

    function resolve(uint256 fuzzedIndex, uint256 fuzzedWaitTime) external {
        emit log_string("ACTOR: resolve start");
        uint256 unresolvedRequestsCount = fetchUnresolvedRequestsCount();
        if (unresolvedRequestsCount == 0) {
            // If no unresolved requests, create a new one, bid on it, and resolve it
            createBidAndResolve(
                fuzzedIndex, // reuse fuzzedIndex for rand uints
                fuzzedIndex,
                fuzzedIndex,
                uint64(fuzzedIndex),
                uint128(fuzzedIndex),
                fuzzedIndex,
                fuzzedIndex,
                fuzzedIndex
            );
            return;
        }

        uint256 targetIndex = bound(fuzzedIndex, 0, unresolvedRequestsCount - 1);
        uint256 actualIndex;
        uint256 currentUnresolvedCount;

        bytes32 requestId;
        for (uint256 i = 0; i < activeRequestIds.length; i++) {
            requestId = activeRequestIds[i];
            if (!hasBeenResolved[activeRequestIds[i]]) {
                if (currentUnresolvedCount == targetIndex) {
                    actualIndex = i;
                    break;
                }
                currentUnresolvedCount++;
            }
        }

        ProofRequestData memory requestData = activeProofRequestData[requestId];
        uint256 bidTimestamp = bidTimestamps[requestId];

        // Wait a variable amount of time between the bid time and the proving deadline
        uint256 provingDeadline = bidTimestamp + requestData.request.provingTime;
        vm.warp(bound(fuzzedWaitTime, bidTimestamp, provingDeadline));

        // Fetch proof & encode opaque submission
        bytes memory opaqueSubmission = baseTest._getGroth16ProofSubmission();

        baseTest._logProofRequest(
            "---------- ACTOR: resolve() selected ProofRequest -------------",
            requestData.request,
            requestData.signature
        );
        baseTest._logAssetBalances(
            "---------- ACTOR: resolve() pre state -------------", address(universalBombetta), alice, bob
        );

        // Resolve the selected active proof request
        vm.prank(bob);
        universalBombetta.resolve(requestData.requestId, opaqueSubmission, bytes32(0));

        baseTest._logAssetBalances(
            "---------- ACTOR: resolve() post state -------------", address(universalBombetta), alice, bob
        );

        hasBeenResolved[requestId] = true;
        resolveCount++;
    }

    /////////////////////////////////////////////////////////////////////////////////////
    /////////////////////////////////// HELPERS /////////////////////////////////////////
    /////////////////////////////////////////////////////////////////////////////////////

    function createAndBid(
        uint256 fuzzedProvingTime,
        uint256 fuzzedTokenAmount,
        uint256 fuzzedMinReward,
        uint64 fuzzedStartTimestamp,
        uint128 fuzzedMinimumStake,
        uint256 fuzzedDeadline,
        uint256 fuzzedWaitTime
    ) public {
        emit log_string("ACTOR: create & bid start");
        (ProofRequest memory request, bytes memory sig) = createAndSignProofRequest(
            fuzzedProvingTime,
            fuzzedTokenAmount,
            fuzzedMinReward,
            fuzzedStartTimestamp,
            fuzzedMinimumStake,
            fuzzedDeadline
        );

        // Wait a variable amount of time between now and the deadline of the auction to bid
        uint256 bidTimestamp = bound(fuzzedWaitTime, request.startAuctionTimestamp, request.endAuctionTimestamp);
        vm.warp(bidTimestamp);

        baseTest._logProofRequest("---------- ACTOR: bid() selected ProofRequest -------------", request, sig);
        baseTest._logAssetBalances(
            "---------- ACTOR: createAndBid() post state -------------", address(universalBombetta), alice, bob
        );

        // Call the bid function as the proof provider/solver
        vm.prank(bob);
        universalBombetta.bid{value: request.minimumStake}(request, sig);

        baseTest._logAssetBalances(
            "---------- ACTOR: createAndBid() post state -------------", address(universalBombetta), alice, bob
        );

        bytes32 requestId = universalBombetta.computeRequestId(request, sig);
        activeRequestIds.push(requestId);
        ProofRequestData memory requestData = ProofRequestData({request: request, signature: sig, requestId: requestId});
        activeProofRequestData[requestId] = requestData;
        bidTimestamps[requestId] = bidTimestamp;

        hasBeenBidOn[allProofRequests.length - 1] = true;
        bidCount++;
    }

    function createBidAndResolve(
        uint256 fuzzedProvingTime,
        uint256 fuzzedTokenAmount,
        uint256 fuzzedMinReward,
        uint64 fuzzedStartTimestamp,
        uint128 fuzzedMinimumStake,
        uint256 fuzzedDeadline,
        uint256 fuzzedBidWaitTime,
        uint256 fuzzedResolveWaitTime
    ) public {
        emit log_string("ACTOR: create, bid & resolve start");
        (ProofRequest memory request, bytes memory sig) = createAndSignProofRequest(
            fuzzedProvingTime,
            fuzzedTokenAmount,
            fuzzedMinReward,
            fuzzedStartTimestamp,
            fuzzedMinimumStake,
            fuzzedDeadline
        );

        // Bid on the request
        uint256 bidTimestamp = bound(fuzzedBidWaitTime, request.startAuctionTimestamp, request.endAuctionTimestamp);
        vm.warp(bidTimestamp);
        vm.prank(bob);
        universalBombetta.bid{value: request.minimumStake}(request, sig);

        bytes32 requestId = universalBombetta.computeRequestId(request, sig);
        activeRequestIds.push(requestId);
        ProofRequestData memory requestData = ProofRequestData({request: request, signature: sig, requestId: requestId});
        activeProofRequestData[requestId] = requestData;
        bidTimestamps[requestId] = bidTimestamp;

        // Resolve the request
        uint256 provingDeadline = bidTimestamp + request.provingTime;
        vm.warp(bound(fuzzedResolveWaitTime, bidTimestamp, provingDeadline));

        bytes memory opaqueSubmission = baseTest._getGroth16ProofSubmission();

        baseTest._logAssetBalances(
            "---------- ACTOR: createBidAndResolve() pre state -------------", address(universalBombetta), alice, bob
        );

        vm.prank(bob);
        universalBombetta.resolve(requestData.requestId, opaqueSubmission, bytes32(0));

        baseTest._logAssetBalances(
            "---------- ACTOR: createBidAndResolve() post state -------------", address(universalBombetta), alice, bob
        );

        hasBeenBidOn[allProofRequests.length - 1] = true;
        hasBeenResolved[requestId] = true;
        bidCount++;
        resolveCount++;
    }

    function fetchUnbidRequestsCount() public view returns (uint256) {
        uint256 unbidCount = 0;
        for (uint256 i = 0; i < allProofRequests.length; i++) {
            if (!hasBeenBidOn[i]) unbidCount++;
        }
        return unbidCount;
    }

    function fetchActiveRequestsIdsLength() public view returns (uint256) {
        return activeRequestIds.length;
    }

    function fetchUnresolvedRequestsCount() public view returns (uint256) {
        uint256 unresolvedCount = 0;
        for (uint256 i = 0; i < activeRequestIds.length; i++) {
            if (!hasBeenResolved[activeRequestIds[i]]) unresolvedCount++;
        }
        return unresolvedCount;
    }

    function _bobTotalPotentialEthObligationFromBiddingOnOutstandingProofRequests() internal view returns (uint256) {
        uint256 totalPotentialObligation = 0;
        for (uint256 i = 0; i < allProofRequests.length; i++) {
            if (!hasBeenBidOn[i]) {
                totalPotentialObligation += allProofRequests[i].request.minimumStake;
            }
        }
        return totalPotentialObligation;
    }

    function _aliceTotalPotentialTokenObligationFromOutstandingProofRequests() internal view returns (uint256) {
        uint256 totalPotentialObligation = 0;
        for (uint256 i = 0; i < allProofRequests.length; i++) {
            if (!hasBeenBidOn[i]) {
                totalPotentialObligation += allProofRequests[i].request.maxRewardAmount;
            }
        }
        return totalPotentialObligation;
    }

    function _initializeFirstOutstandingProofRequest() internal view returns (ProofRequest memory, bytes memory) {
        /// Set up the proof request data
        // Metadata.extraData
        UniversalBombetta.VerifierDetails memory verifierDetails = UniversalBombetta.VerifierDetails({
            verifier: address(verifierG16),
            selector: verifierG16.verifyProof.selector,
            isShaCommitment: false,
            publicInputsOffset: 256,
            publicInputsLength: 32,
            hasPartialCommitmentResultCheck: false,
            submittedPartialCommitmentResultOffset: 0,
            submittedPartialCommitmentResultLength: 0,
            predeterminedPartialCommitment: bytes32(0)
        });

        // ProofRequest
        ProofRequest memory request = ProofRequest({
            signer: alice,
            market: address(universalBombetta),
            nonce: 1,
            token: address(testToken),
            maxRewardAmount: 1000 ether, // 1000 tokens
            minRewardAmount: 0,
            minimumStake: 1 ether,
            startAuctionTimestamp: uint64(block.timestamp),
            endAuctionTimestamp: uint64(block.timestamp + 1000),
            provingTime: 1 days,
            publicInputsCommitment: keccak256(abi.encode(33)),
            extraData: abi.encode(verifierDetails)
        });

        bytes memory sig = baseTest._getBombettaSignature(address(universalBombetta), request, ALICE_PK);

        return (request, sig);
    }
}
