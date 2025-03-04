// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.23;

import "forge-std/Script.sol";
import "forge-std/Test.sol";
import "test/mocks/ERC20Mock.sol";

contract Approvals is Script, Test {
    // local anvil instance must fork from ethereum main-net for permit2
    string RPC_URL = vm.envString("ETH_HOLESKY_RPC_URL");
    // test reward token contract
    ERC20Mock public immutable rewardToken = ERC20Mock(0x89fF1B147026815cf497AA45D4FDc2DF51Ed7f00);
    // permit2 contract
    address public immutable PERMIT2 = address(0x000000000022D473030F116dDEE9F6B43aC78BA3);
    // bombetta contract
    address public immutable bombetta = address(0x561D39042B6FC0D5fA78DaE730468e2C339B5eD3);
    // porchetta contract
    address public immutable porchetta = address(0x554D0292CD1E740123b360DD6D10037De00a0Bd7);

    function run() public {
        uint256 deployerPrivateKey = vm.envUint("REQUESTER_PRIVATE_KEY");
        vm.createSelectFork(RPC_URL);
        vm.startBroadcast(deployerPrivateKey);

        // requester max approves the permit2 and porchetta contracts for rewardToken
        rewardToken.approve(PERMIT2, type(uint256).max);
        rewardToken.approve(bombetta, type(uint256).max);
        rewardToken.approve(porchetta, type(uint256).max);
        vm.stopBroadcast();

        deployerPrivateKey = vm.envUint("PROVIDER_PRIVATE_KEY");
        vm.createSelectFork(RPC_URL);
        vm.startBroadcast(deployerPrivateKey);

        // provider max approves the permit2 contract for rewardToken
        rewardToken.approve(PERMIT2, type(uint256).max);
        vm.stopBroadcast();
    }
}