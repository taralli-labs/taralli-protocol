// SPDX-License-Identifier: UNLICENSED
pragma solidity ^0.8.23;

import "forge-std/Script.sol";
import "forge-std/Test.sol";
import "test/mocks/ERC20Mock.sol";

contract CheckBalanceAndApproval is Script, Test {
    // local anvil instance must fork from ethereum main-net for permit2
    string RPC_URL = vm.envString("ETH_SEPOLIA_RPC_URL");
    // test reward token contract
    ERC20Mock public immutable rewardToken = ERC20Mock(0xb54061f59AcF94f86ee414C9a220aFFE8BbE6B35);
    // test stake token contract
    ERC20Mock public immutable stakeToken = ERC20Mock(0x3D48eB902f38fCF16C2fD9F42cb088d301D16c94);
    // permit2 contract
    address public immutable PERMIT2 = address(0x000000000022D473030F116dDEE9F6B43aC78BA3);
    // porchetta contract
    address public immutable porchetta = address(0x67445680c74Fb82C46421374554e402e72E9e5d1);
    // bombetta contract
    address public immutable bombetta = address(0x6209431B6C8F38471dc65564Be2Fd08298705BBD);
    // address of requester
    address public immutable requester = address(0xC342071B52566FcD2a8D47a0b18A5884c4a0627f);
    // address of provider
    address public immutable provider = address(0x4070Af7fc9090Ec323330dDed79159E8740b5158);

    function run() public {
        uint256 deployerPrivateKey = vm.envUint("LOCAL_PRIVATE_KEY");
        vm.createSelectFork(RPC_URL);
        vm.startBroadcast(deployerPrivateKey);

        // token balances
        emit log_named_uint("test token balance of requester", rewardToken.balanceOf(requester));
        emit log_named_uint("test token balance of provider", rewardToken.balanceOf(provider));
        emit log_named_uint("test token balance of bombetta", rewardToken.balanceOf(bombetta));
        emit log_named_uint("test token balance of porchetta", rewardToken.balanceOf(porchetta));
        emit log_named_uint("stake token balance of requester", stakeToken.balanceOf(requester));
        emit log_named_uint("stake token balance of provider", stakeToken.balanceOf(provider));
        emit log_named_uint("stake token balance of bombetta", stakeToken.balanceOf(bombetta));
        emit log_named_uint("stake token balance of porchetta", stakeToken.balanceOf(porchetta));
        // eth balances
        emit log_named_uint("bombetta eth balance", bombetta.balance);
        emit log_named_uint("porchetta eth balance", porchetta.balance);
        // allowances
        emit log_named_uint("permit2 requester allowance", rewardToken.allowance(requester, PERMIT2));
        emit log_named_uint("permit2 provider allowance", rewardToken.allowance(provider, PERMIT2));
        emit log_named_uint("bombetta requester allowance", rewardToken.allowance(requester, bombetta));
        emit log_named_uint("bombetta provider allowance", rewardToken.allowance(provider, bombetta));
        emit log_named_uint("porchetta requester allowance", rewardToken.allowance(requester, PERMIT2));
        emit log_named_uint("porchetta provider allowance", rewardToken.allowance(provider, PERMIT2));        

        vm.stopBroadcast();
    }
}
