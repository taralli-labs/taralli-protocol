// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

import "forge-std/Test.sol";
import "src/UniversalBombetta.sol";
import "src/verifiers/SimpleGroth16Verifier.sol";
import "src/interfaces/IPermit2.sol";
import "src/libraries/BombettaTypes.sol";
import "./mocks/ERC20Mock.sol";

contract BaseTest is Test {
    // permit2 interface
    IPermit2 permit2;
    // Bombetta contract(s)
    UniversalBombetta universalBombetta;
    // verifier contract(s)
    SimpleGroth16Verifier verifierG16;
    // test tokens for rewards
    ERC20Mock testToken;

    // test accounts
    uint256 ALICE_PK = 1;
    uint256 BOB_PK = 2;
    address alice = vm.addr(ALICE_PK);
    address bob = vm.addr(BOB_PK);

    bytes32 public BOMBETTA_MARKET_WITNESS_TYPEHASH;

    string RPC_ETH_HOLESKY = vm.envString("ETH_HOLESKY_RPC_URL");
    //string LOCAL_RPC = vm.envString("ETH_LOCAL_RPC_URL");

    function _setUp() internal {
        // fetch network state
        uint256 forkId = vm.createFork(RPC_ETH_HOLESKY, 2217349);
        //uint256 forkId = vm.createFork(LOCAL_RPC);
        vm.selectFork(forkId);

        // set canonical permit 2 addr
        permit2 = IPermit2(0x000000000022D473030F116dDEE9F6B43aC78BA3);

        // deploy verifier(s)
        verifierG16 = new SimpleGroth16Verifier();

        // deploy bombetta(s)
        universalBombetta = new UniversalBombetta(permit2);

        // set typehash for permit2 signatures
        BOMBETTA_MARKET_WITNESS_TYPEHASH = keccak256(
            abi.encodePacked(
                universalBombetta.PERMIT_TRANSFER_FROM_WITNESS_TYPEHASH_STUB(),
                universalBombetta.FULL_PROOF_REQUEST_WITNESS_TYPE_STRING_STUB()
            )
        );

        // deal eth to prover(s) for stake
        vm.deal(bob, 10 ether);
        vm.deal(alice, 10 ether);

        // deploy mock erc20 token
        testToken = new ERC20Mock("Test Token", "TEST", 18);
        // mint proof requester some reward tokens
        testToken.mint(alice, 100000 ether);
        // mint proof provider some tokens for stake
        testToken.mint(bob, 100000 ether);

        // max approve permit2 contract on proof requester accounts
        vm.prank(alice);
        testToken.approve(address(permit2), type(uint256).max);
    }

    /////////////////////////////////// HELPERS /////////////////////////////////////////

    function _getGroth16ProofSubmission() public view returns (bytes memory) {
        // Read proof data from proof.json
        string memory proofJson = vm.readFile("./test-proof-data/groth16/proof.json");

        uint256[] memory pi_a = vm.parseJsonUintArray(proofJson, "$.pi_a");
        uint256[2] memory a;
        a[0] = pi_a[0];
        a[1] = pi_a[1];

        //string[] memory pi_b_str = vm.readUintArray("$.b")
        uint256[2][2] memory b;

        uint256[] memory row0 = vm.parseJsonUintArray(proofJson, "$.pi_b[0]");
        b[0] = [row0[1], row0[0]];
        uint256[] memory row1 = vm.parseJsonUintArray(proofJson, "$.pi_b[1]");
        b[1] = [row1[1], row1[0]];

        uint256[] memory pi_c = vm.parseJsonUintArray(proofJson, "$.pi_c");
        uint256[2] memory c;
        c[0] = pi_c[0];
        c[1] = pi_c[1];

        // Read public input data from public.json
        string memory publicJson = vm.readFile("./test-proof-data/groth16/public.json");

        uint256[] memory parsedPubSignals = vm.parseJsonUintArray(publicJson, "$");
        uint256[1] memory pubSignals;
        pubSignals[0] = parsedPubSignals[0];

        return abi.encode(a, b, c, pubSignals);
    }

    struct ModableTestSubmission {
        uint256[2] _pA;
        uint256[2][2] _pB;
        uint256[2] _pC;
        uint256[1] _pubSignals;
    }

    function _getModableGroth16ProofSubmission() internal view returns (ModableTestSubmission memory) {
        // Read proof data from proof.json
        string memory proofJson = vm.readFile("./test-proof-data/groth16/proof.json");

        uint256[] memory pi_a = vm.parseJsonUintArray(proofJson, "$.pi_a");
        uint256[2] memory a;
        a[0] = pi_a[0];
        a[1] = pi_a[1];

        uint256[2][2] memory b;

        uint256[] memory row0 = vm.parseJsonUintArray(proofJson, "$.pi_b[0]");
        b[0] = [row0[1], row0[0]];
        uint256[] memory row1 = vm.parseJsonUintArray(proofJson, "$.pi_b[1]");
        b[1] = [row1[1], row1[0]];

        uint256[] memory pi_c = vm.parseJsonUintArray(proofJson, "$.pi_c");
        uint256[2] memory c;
        c[0] = pi_c[0];
        c[1] = pi_c[1];

        // Read public input data from public.json
        string memory publicJson = vm.readFile("./test-proof-data/groth16/public.json");

        uint256[] memory parsedPubSignals = vm.parseJsonUintArray(publicJson, "$");
        uint256[1] memory pubSignals;
        pubSignals[0] = parsedPubSignals[0];

        // Return the submission struct
        return ModableTestSubmission({_pA: a, _pB: b, _pC: c, _pubSignals: pubSignals});
    }

    function _getBombettaSignature(address market, ProofRequest memory request, uint256 privKey)
        public
        view
        returns (bytes memory)
    {
        // Create permit
        ISignatureTransfer.PermitTransferFrom memory permit = ISignatureTransfer.PermitTransferFrom({
            permitted: ISignatureTransfer.TokenPermissions({token: request.token, amount: request.maxRewardAmount}),
            nonce: request.nonce,
            deadline: request.endAuctionTimestamp
        });

        // Create witness
        ProofRequest memory proofRequestWitness = ProofRequest({
            signer: request.signer,
            market: request.market,
            nonce: request.nonce,
            token: request.token,
            maxRewardAmount: request.maxRewardAmount,
            minRewardAmount: request.minRewardAmount,
            minimumStake: request.minimumStake,
            startAuctionTimestamp: request.startAuctionTimestamp,
            endAuctionTimestamp: request.endAuctionTimestamp,
            provingTime: request.provingTime,
            publicInputsCommitment: request.publicInputsCommitment,
            extraData: request.extraData
        });
        bytes32 witness = universalBombetta.computeWitnessHash(proofRequestWitness);

        return _getPermitWitnessTransferSignatureForProofMarket(
            address(market), permit, witness, BOMBETTA_MARKET_WITNESS_TYPEHASH, privKey
        );
    }

    function _getPermitWitnessTransferSignatureForProofMarket(
        address proofMarketAddr,
        ISignatureTransfer.PermitTransferFrom memory permit,
        bytes32 witness,
        bytes32 typeHash,
        uint256 privateKey
    ) internal view returns (bytes memory sig) {
        bytes32 tokenPermissionsHash =
            keccak256(abi.encode(universalBombetta.TOKEN_PERMISSIONS_TYPEHASH(), permit.permitted));

        bytes32 dataHash = keccak256(
            abi.encode(typeHash, tokenPermissionsHash, proofMarketAddr, permit.nonce, permit.deadline, witness)
        );

        bytes32 msgHash = _hashTypedData(permit2.DOMAIN_SEPARATOR(), dataHash);

        (uint8 v, bytes32 r, bytes32 s) = vm.sign(privateKey, msgHash);
        return bytes.concat(r, s, bytes1(v));
    }

    /// @notice Creates an EIP-712 typed data hash
    function _hashTypedData(bytes32 domainSeparator, bytes32 dataHash) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked("\x19\x01", domainSeparator, dataHash));
    }

    function _logAssetBalances(string memory note, address provingMarket, address requester, address provider) public {
        uint256 marketTokenBalance = testToken.balanceOf(address(provingMarket));
        uint256 requesterTokenBalance = testToken.balanceOf(requester);
        uint256 providerTokenBalance = testToken.balanceOf(provider);
        emit log_string(note);
        emit log_named_uint("MARKET eth balance     ", provingMarket.balance);
        emit log_named_uint("MARKET token balance   ", marketTokenBalance);
        emit log_named_uint("REQUESTER eth balance  ", requester.balance);
        emit log_named_uint("REQUESTER token balance", requesterTokenBalance);
        emit log_named_uint("PROVIDER eth balance   ", provider.balance);
        emit log_named_uint("PROVIDER token balance ", providerTokenBalance);
    }

    function _logProofRequest(string memory note, ProofRequest memory request, bytes memory signature) public {
        emit log_string(note);
        emit log_named_address("market", request.market);
        emit log_named_uint("nonce", request.nonce);
        emit log_named_address("token", request.token);
        emit log_named_uint("amount", request.maxRewardAmount);
        emit log_named_uint("minReward", request.minRewardAmount);
        emit log_named_uint("minimumStake", request.minimumStake);
        emit log_named_uint("startAuctionTimestamp", request.startAuctionTimestamp);
        emit log_named_uint("endAuctionTimestamp", request.endAuctionTimestamp);
        emit log_named_uint("provingTime", request.provingTime);
        emit log_named_bytes32("publicInputsCommitment", request.publicInputsCommitment);
        emit log_named_bytes("extraData", request.extraData);
        emit log_named_bytes("signature", signature);
    }
}
