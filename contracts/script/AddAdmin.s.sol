// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.23;

import "forge-std/Script.sol";
import "forge-std/Test.sol";
import "test/mocks/ERC20Mock.sol";

contract AddAdmin is Script, Test {
    // local anvil instance must fork from ethereum main-net for permit2
    string RPC_URL = vm.envString("ETH_HOLESKY_RPC_URL");
    // test reward token contract
    ERC20Mock public immutable rewardToken = ERC20Mock(0x1b47Ec8Ed7F7E4358325d9627D42C7feC10f5b91);
    // new admin address to give minting privileges to
    address public new_admin_address = address(0x5eD981350EEFC258c09a7a7d54b197db0aB22C5b);

    function run() public {
        uint256 deployerPrivateKey = vm.envUint("LOCAL_PRIVATE_KEY");
        vm.createSelectFork(RPC_URL);
        vm.startBroadcast(deployerPrivateKey);

        // add new admin address
        rewardToken.addAdmin(new_admin_address);
    }
}
