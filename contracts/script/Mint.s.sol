// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.23;

import "forge-std/Script.sol";
import "forge-std/Test.sol";
import "test/mocks/ERC20Mock.sol";

contract Mint is Script, Test {
    // local anvil instance must fork from ethereum main-net for permit2
    string RPC_URL = vm.envString("ETH_HOLESKY_RPC_URL");
    // test reward token contract
    ERC20Mock public immutable rewardToken = ERC20Mock(0x89fF1B147026815cf497AA45D4FDc2DF51Ed7f00);

    function run() public {
        uint256 deployerPrivateKey = vm.envUint("LOCAL_PRIVATE_KEY");
        vm.createSelectFork(RPC_URL);
        vm.startBroadcast(deployerPrivateKey);

        // mint 10 mil tokens
        rewardToken.mint(address(0xC342071B52566FcD2a8D47a0b18A5884c4a0627f), 10000000 ether);
    }
}
