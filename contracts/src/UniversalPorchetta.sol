// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

import "solady/utils/ECDSA.sol";
import {SafeTransferLib as STL} from "solady/utils/SafeTransferLib.sol";

import "./abstract/Porchetta.sol";
import "src/interfaces/IPermit2.sol";
import "./libraries/PorchettaTypes.sol";
import "./libraries/Errors.sol";

/// @title  UniversalPorchetta, a compute offer based permission-less verifiable compute marketplace
/// @author Taralli Labs
contract UniversalPorchetta is Porchetta {
    using ECDSA for bytes32;

    /////////////////////////////// TYPES ////////////////////////////////////

    struct ActiveProofOffer {
        // address of the proof provider obligated to resolve the proof offer.
        address provider;
        // address of the requester that bid on the proof offer.
        address requester;
        // deadline timestamp the proof provider must resolve the offer by.
        uint256 resolutionDeadline;
        // reward token.
        address rewardToken;
        // offer reward token amount.
        uint256 rewardAmount;
        // stake token used as collateral for the proof offer.
        address stakeToken;
        // amount of `stakeToken` proof provider has staked.
        uint256 stakeAmount;
        // hash of all knowledge the proof provider commits to within the opaque submission excluding the proof itself.
        bytes32 inputsCommitment;
        // data specific to verification.
        bytes verifierDetails;
    }

    struct VerifierDetails {
        // address of the verifier contract required by the requester
        address verifier;
        // fn selector of the verifying function required by the requester
        bytes4 selector;
        // bool to chose between keccak256 or sha256 for commitments, true = sha256, false = keccak256
        bool isShaCommitment;
        // offset of inputs field within the proof submission data (opaqueSubmission)
        uint256 inputsOffset;
        // length of inputs field within the proof submission data (opaqueSubmission)
        uint256 inputsLength;
    }

    ////////////////////////////// CONSTANTS /////////////////////////////////

    // Canonical Permit2 contract
    IPermit2 public immutable PERMIT2;
    // permit2 permitWitnessTransferFrom() type hashes
    bytes32 public constant PERMIT_TRANSFER_FROM_TYPEHASH = keccak256(
        "PermitTransferFrom(TokenPermissions permitted,address spender,uint256 nonce,uint256 deadline)TokenPermissions(address token,uint256 amount)"
    );
    bytes32 public constant TOKEN_PERMISSIONS_TYPEHASH = keccak256("TokenPermissions(address token,uint256 amount)");
    string public constant PERMIT_TRANSFER_FROM_WITNESS_TYPEHASH_STUB =
        "PermitWitnessTransferFrom(TokenPermissions permitted,address spender,uint256 nonce,uint256 deadline,";
    string public constant FULL_PROOF_OFFER_WITNESS_TYPE_STRING_STUB =
        "ProofOffer witness)TokenPermissions(address token,uint256 amount)ProofOffer(address signer,address market,uint256 nonce,address rewardToken,uint256 rewardAmount,address stakeToken,uint256 stakeAmount,uint64 startAuctionTimestamp,uint64 endAuctionTimestamp,uint32 provingTime,bytes32 inputsCommitment,bytes extraData)";
    bytes public constant PROOF_OFFER_WITNESS_TYPE =
        "ProofOffer(address signer,address market,uint256 nonce,address rewardToken,uint256 rewardAmount,address stakeToken,uint256 stakeAmount,uint64 startAuctionTimestamp,uint64 endAuctionTimestamp,uint32 provingTime,bytes32 inputsCommitment,bytes extraData)";
    bytes32 public constant PROOF_OFFER_WITNESS_TYPE_HASH = keccak256(PROOF_OFFER_WITNESS_TYPE);

    /////////////////////////////// STORAGE //////////////////////////////////

    // mapping to active proof offer data
    // offerId -> ActiveProofOffer
    mapping(bytes32 => ActiveProofOffer) public activeProofOfferData;

    //////////////////////////// CONSTRUCTOR /////////////////////////////////

    constructor(IPermit2 _permit) {
        PERMIT2 = _permit;
    }

    ///////////////////////// EXTERNAL FUNCTIONS /////////////////////////////

    /// @notice Place a bid for a signed proof offer.
    /// @param offer The proof offer that is being bid upon.
    /// @param signature The signature of the proof offer.
    /// @return rewardToken The address of the reward token
    ///         rewardAmount The token reward amount available upon resolution.
    ///         resolutionDeadline The timestamp defining when the proof offer must be resolved.
    function bid(ProofOffer calldata offer, bytes calldata signature)
        external
        override
        returns (address, uint256, uint256)
    {
        // check offer data
        if (block.timestamp < offer.startAuctionTimestamp) revert InvalidOffer();
        if (block.timestamp > offer.endAuctionTimestamp) revert InvalidOffer();
        // compute offer ID
        bytes32 offerId = computeOfferId(offer);
        // assert that only 1 bid can be placed for a given offer
        if (activeProofOfferData[offerId].requester != address(0)) revert AuctionEnded();

        // Build permit struct
        ISignatureTransfer.PermitTransferFrom memory permit = ISignatureTransfer.PermitTransferFrom({
            permitted: ISignatureTransfer.TokenPermissions({token: offer.stakeToken, amount: offer.stakeAmount}),
            nonce: offer.nonce,
            deadline: offer.endAuctionTimestamp
        });

        // Generate witness hash
        bytes32 witness = computeWitnessHash(offer);

        // transfer the proof provider's token stake to this contract &
        // validate signature of the offer in the process
        PERMIT2.permitWitnessTransferFrom(
            permit,
            ISignatureTransfer.SignatureTransferDetails({to: address(this), requestedAmount: offer.stakeAmount}),
            offer.signer,
            witness,
            FULL_PROOF_OFFER_WITNESS_TYPE_STRING_STUB,
            signature
        );

        // transfer the reward to this contract
        STL.safeTransferFrom(offer.rewardToken, msg.sender, address(this), offer.rewardAmount);

        // store bid data for resolution
        uint256 resolutionDeadline = block.timestamp + offer.provingTime;
        activeProofOfferData[offerId] = ActiveProofOffer({
            provider: offer.signer,
            requester: msg.sender,
            resolutionDeadline: resolutionDeadline,
            rewardToken: offer.rewardToken,
            rewardAmount: offer.rewardAmount,
            stakeToken: offer.stakeToken,
            stakeAmount: offer.stakeAmount,
            inputsCommitment: offer.inputsCommitment,
            verifierDetails: offer.extraData
        });

        emit Bid(
            offer.signer,
            offerId,
            offer.rewardToken,
            offer.rewardAmount,
            offer.stakeToken,
            offer.stakeAmount,
            msg.sender
        );

        return (offer.rewardToken, offer.rewardAmount, resolutionDeadline);
    }

    /// @notice Resolve a bid for a signed proof offer.
    /// @param offerId The offerId associated with the proof offer being resolved.
    /// @param opaqueSubmission The opaque data that will be decoded by the market contract and passed
    ///                         to the verifier. Empty if the deadline has been reached and the provider
    ///                         is being slashed.
    function resolve(bytes32 offerId, bytes calldata opaqueSubmission)
        external
        override
        returns (bool providerResolved)
    {
        // load active proof offer data
        ActiveProofOffer memory activeProofOffer = activeProofOfferData[offerId];
        address rewardToken = activeProofOffer.rewardToken;
        address stakeToken = activeProofOffer.stakeToken;
        // if the offer has a valid deadline, check proof validity
        if (block.timestamp <= activeProofOffer.resolutionDeadline) {
            if (msg.sender != activeProofOffer.provider) {
                // alternative proof provider is not allowed until resolutionDeadline has passed
                revert InvalidResolver();
            }
            providerResolved = _checkProofSubmission(
                activeProofOffer.inputsCommitment, activeProofOffer.verifierDetails, opaqueSubmission
            );
            // if proof is valid then reward provider
            if (providerResolved) {
                // reward prover
                address prover = activeProofOffer.provider;
                STL.safeTransfer(stakeToken, prover, activeProofOffer.stakeAmount);
                STL.safeTransfer(rewardToken, prover, activeProofOffer.rewardAmount);
            } else {
                // invalid proof, slash prover
                address requester = activeProofOffer.requester;
                STL.safeTransfer(stakeToken, requester, activeProofOffer.stakeAmount);
                STL.safeTransfer(rewardToken, requester, activeProofOffer.rewardAmount);
            }
        } else {
            // prover intent expired, slash prover
            address requester = activeProofOffer.requester;
            STL.safeTransfer(stakeToken, requester, activeProofOffer.stakeAmount);
            STL.safeTransfer(rewardToken, requester, activeProofOffer.rewardAmount);
        }

        emit Resolve(activeProofOffer.provider, offerId, msg.sender);
    }

    ///////////////////////// INTERNAL FUNCTIONS /////////////////////////////

    /// @dev check the correctness of the submitted proof during execution of resolve()
    function _checkProofSubmission(
        bytes32 inputsCommitment,
        bytes memory verifierDetails,
        bytes calldata opaqueSubmission
    ) internal returns (bool) {
        VerifierDetails memory vd = abi.decode(verifierDetails, (VerifierDetails));

        // check inputs if needed
        if (vd.inputsLength > 0) {
            // extract submitted inputs field within opaqueSubmission data
            if (vd.inputsOffset + vd.inputsLength > opaqueSubmission.length) {
                revert InvalidInputsCommitmentField();
            }
            bytes memory submittedInputs = opaqueSubmission[vd.inputsOffset:vd.inputsOffset + vd.inputsLength];

            // Check submitted vs expected input(s) commitments
            if (vd.isShaCommitment) {
                if (inputsCommitment != sha256(submittedInputs)) {
                    return false;
                }
            } else {
                if (inputsCommitment != keccak256(submittedInputs)) {
                    return false;
                }
            }
        }
        return _callVerifier(vd.verifier, vd.selector, opaqueSubmission);
    }

    /// @dev Perform the call to the verifier and return the result of the call
    function _callVerifier(address verifier, bytes4 selector, bytes calldata opaqueSubmission)
        internal
        returns (bool)
    {
        bool success;
        assembly {
            let ptr := mload(0x40)
            // Store the function selector
            mstore(ptr, selector)
            // Copy the opaqueSubmission data right after the function selector
            calldatacopy(add(ptr, 4), opaqueSubmission.offset, opaqueSubmission.length)
            let returndata_size := 32
            let returndata := add(ptr, add(opaqueSubmission.length, 4))
            success := call(gas(), verifier, 0, ptr, add(opaqueSubmission.length, 4), returndata, returndata_size)
        }
        return success;
    }

    /// @dev hashes the proof offer for use as the offer ID in mapping `activeProofOfferData`
    function computeOfferId(ProofOffer calldata offer) public pure returns (bytes32) {
        return keccak256(
            abi.encode(
                offer.signer,
                offer.market,
                offer.nonce,
                offer.rewardToken,
                offer.rewardAmount,
                offer.stakeToken,
                offer.stakeAmount,
                offer.startAuctionTimestamp,
                offer.endAuctionTimestamp,
                offer.provingTime,
                offer.inputsCommitment,
                offer.extraData
            )
        );
    }

    /// @notice hash a ProofRequest, used as witness to the signature
    /// @param proofOfferWitness The ProofOffer object to hash
    function computeWitnessHash(ProofOffer memory proofOfferWitness) public pure returns (bytes32) {
        return keccak256(
            abi.encode(
                PROOF_OFFER_WITNESS_TYPE_HASH,
                proofOfferWitness.signer,
                proofOfferWitness.market,
                proofOfferWitness.nonce,
                proofOfferWitness.rewardToken,
                proofOfferWitness.rewardAmount,
                proofOfferWitness.stakeToken,
                proofOfferWitness.stakeAmount,
                proofOfferWitness.startAuctionTimestamp,
                proofOfferWitness.endAuctionTimestamp,
                proofOfferWitness.provingTime,
                proofOfferWitness.inputsCommitment,
                keccak256(proofOfferWitness.extraData)
            )
        );
    }

    /// @dev computes the digest the signer signed in order to perform the ec recover operation.
    function computePermitDigest(ISignatureTransfer.PermitTransferFrom memory permit, bytes32 witness)
        external
        view
        returns (bytes32)
    {
        bytes32 typeHash = keccak256(
            abi.encodePacked(PERMIT_TRANSFER_FROM_WITNESS_TYPEHASH_STUB, FULL_PROOF_OFFER_WITNESS_TYPE_STRING_STUB)
        );

        bytes32 tokenPermissionsHash = keccak256(abi.encode(TOKEN_PERMISSIONS_TYPEHASH, permit.permitted));

        bytes32 dataHash =
            keccak256(abi.encode(typeHash, tokenPermissionsHash, address(this), permit.nonce, permit.deadline, witness));

        return _hashTypedData(PERMIT2.DOMAIN_SEPARATOR(), dataHash);
    }

    /// @notice Creates an EIP-712 typed data hash
    function _hashTypedData(bytes32 domainSeparator, bytes32 dataHash) internal pure returns (bytes32) {
        return keccak256(abi.encodePacked("\x19\x01", domainSeparator, dataHash));
    }
}
