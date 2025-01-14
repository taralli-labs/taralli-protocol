// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.23;

import "forge-std/Script.sol";
import "forge-std/Test.sol";
import "src/interfaces/IPermit2.sol";
import "test/mocks/ERC20Mock.sol";

contract PermitApprove is Script, Test {
    // local anvil instance must fork from ethereum main-net for permit2
    string RPC_URL = vm.envString("ETH_HOLESKY_RPC_URL");
    // Canonical Permit2 contract
    address public immutable PERMIT2 = address(0x000000000022D473030F116dDEE9F6B43aC78BA3);
    //address public immutable bombetta = address(0x0c5CFe655ee594Ea9748Af27D5a147D00f86665b);
    // Test token contract
    ERC20Mock public immutable rewardToken = ERC20Mock(0x89fF1B147026815cf497AA45D4FDc2DF51Ed7f00);

    function run() public {
        uint256 deployerPrivateKey = vm.envUint("LOCAL_PRIVATE_KEY");
        vm.createSelectFork(RPC_URL);
        vm.startBroadcast(deployerPrivateKey);

        // caller max approves permit2 contract for rewardToken
        rewardToken.approve(PERMIT2, type(uint256).max);
    }
}
