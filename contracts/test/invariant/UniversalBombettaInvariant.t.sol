// SPDX-License-Identifier: MIT
pragma solidity ^0.8.23;

import "forge-std/StdInvariant.sol";
import "src/libraries/BombettaTypes.sol";
import "./actors/BombettaActor.sol";
import "../BaseTest.sol";

contract UniversalBombettaInvariant is BaseTest {
    BombettaActor public actor;

    function setUp() external {
        _setUp();

        // Deploy actor
        actor = new BombettaActor(
            alice, ALICE_PK, bob, address(this), address(testToken), address(universalBombetta), address(verifierG16)
        );

        // target proof provider address (bob)
        targetSender(bob);
        // Target only the actor contract
        targetContract(address(actor));
        // Target only the (createAndSignProofRequest + bid + resolve) functions
        bytes4[] memory selectors = new bytes4[](3);
        selectors[0] = actor.createAndSignProofRequest.selector;
        selectors[1] = actor.bid.selector;
        selectors[2] = actor.resolve.selector;
        targetSelector(FuzzSelector({addr: address(actor), selectors: selectors}));
        emit log_named_uint("INVARIANT SETUP FINISHED", 0);
    }

    function invariant_solvency() external {
        uint256 realTokenBalance = testToken.balanceOf(address(universalBombetta));
        uint256 perceivedTokenbalance = _computeTotalExpectedTokenBalanceOfBombetta();
        uint256 realEthBalance = address(universalBombetta).balance;
        uint256 perceivedEthBalance = _computeTotalEthBalanceOfBombetta();
        emit log_named_uint("-------- TOKEN SOLVENCY --------", 0);
        emit log_named_uint("LEFT: testToken.balanceOf(address(universalBombetta))", realTokenBalance);
        emit log_named_uint("RIGHT: _computeTotalTokenBalanceOfBombetta()         ", perceivedTokenbalance);
        emit log_named_uint("-------- ETH SOLVENCY --------", 0);
        emit log_named_uint("LEFT: address(universalBombetta).balance  ", realEthBalance);
        emit log_named_uint("RIGHT: _computeTotalEthBalanceOfBombetta()", perceivedEthBalance);
        // invariants
        assertEq(realTokenBalance, perceivedTokenbalance);
        assertEq(address(universalBombetta).balance, _computeTotalEthBalanceOfBombetta());
    }

    /////////////////////////////////// HELPERS /////////////////////////////////////////

    function _computeTotalExpectedTokenBalanceOfBombetta() internal view returns (uint256) {
        uint256 tokenBalanceTotal;
        for (uint256 i = 0; i < actor.fetchActiveRequestsIdsLength(); i++) {
            bytes32 requestId = actor.activeRequestIds(i);
            if (!actor.hasBeenResolved(requestId)) {
                (,,,, uint256 requestReward,,,) = universalBombetta.activeProofRequestData(requestId);
                tokenBalanceTotal += requestReward;
            }
        }
        return tokenBalanceTotal;
    }

    function _computeTotalEthBalanceOfBombetta() internal view returns (uint256) {
        uint256 ethBalanceTotal;
        for (uint256 i = 0; i < actor.fetchActiveRequestsIdsLength(); i++) {
            bytes32 requestId = actor.activeRequestIds(i);
            if (!actor.hasBeenResolved(requestId)) {
                (,,,,, uint256 providerStake,,) = universalBombetta.activeProofRequestData(requestId);
                ethBalanceTotal += providerStake;
            }
        }
        return ethBalanceTotal;
    }
}
