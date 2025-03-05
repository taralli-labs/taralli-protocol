// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.23;

import "forge-std/Script.sol";
import "forge-std/Test.sol";
import "test/mocks/ERC20Mock.sol";

contract Mint is Script, Test {
    // local anvil instance must fork from ethereum main-net for permit2
    string RPC_URL = vm.envString("ETH_SEPOLIA_RPC_URL");
    // test reward token contract
    ERC20Mock public immutable rewardToken = ERC20Mock(0xb54061f59AcF94f86ee414C9a220aFFE8BbE6B35);

    function run() public {
        uint256 deployerPrivateKey = vm.envUint("REQUESTER_PRIVATE_KEY");
        vm.createSelectFork(RPC_URL);
        vm.startBroadcast(deployerPrivateKey);

        // mint 10 mil tokens
        rewardToken.mint(address(0x4070Af7fc9090Ec323330dDed79159E8740b5158), 10000000 ether);
    }
}