// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.23;

import "forge-std/Script.sol";
import "forge-std/Test.sol";
import "test/mocks/ERC20Mock.sol";

contract CheckBalanceAndApproval is Script, Test {
    // local anvil instance must fork from ethereum main-net for permit2
    string RPC_URL = vm.envString("ETH_HOLESKY_RPC_URL");
    // test reward token contract
    ERC20Mock public immutable rewardToken = ERC20Mock(0x89fF1B147026815cf497AA45D4FDc2DF51Ed7f00);
    // permit2 contract
    address public immutable PERMIT2 = address(0x000000000022D473030F116dDEE9F6B43aC78BA3);

    function run() public {
        uint256 deployerPrivateKey = vm.envUint("LOCAL_PRIVATE_KEY");
        vm.createSelectFork(RPC_URL);
        vm.startBroadcast(deployerPrivateKey);

        emit log_named_address("caller", msg.sender);

        // check token balance of caller
        uint256 balance = rewardToken.balanceOf(msg.sender);
        emit log_named_uint("balance of", balance);
        // check appproval amounts for permit2
        uint256 allowance = rewardToken.allowance(address(0xC342071B52566FcD2a8D47a0b18A5884c4a0627f), PERMIT2);
        emit log_named_uint("allowance", allowance);
        vm.stopBroadcast();
    }
}
