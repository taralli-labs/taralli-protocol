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
    // permit2 contract
    address public immutable PERMIT2 = address(0x000000000022D473030F116dDEE9F6B43aC78BA3);
    // porchetta contract
    address public immutable porchetta = address(0x5Ac1172921d0CdfFF58B59E23f8DeAE86bDca565);
    // bombetta contract
    address public immutable bombetta = address(0x4bE2653870EBCAda3C99D03C63e265fD57882e3b);
    // address of requester
    address public immutable requester = address(0xC342071B52566FcD2a8D47a0b18A5884c4a0627f);
    // address of provider
    address public immutable provider = address(0x4070Af7fc9090Ec323330dDed79159E8740b5158);


    function run() public {
        uint256 deployerPrivateKey = vm.envUint("LOCAL_PRIVATE_KEY");
        vm.createSelectFork(RPC_URL);
        vm.startBroadcast(deployerPrivateKey);

        emit log_named_address("caller", msg.sender);

        // check token balance of caller
        uint256 balance = rewardToken.balanceOf(msg.sender);
        emit log_named_uint("test token balance of caller", balance);
        // check token balance of requester
        uint256 balance2 = rewardToken.balanceOf(requester);
        emit log_named_uint("test token balance of requester", balance2);
        // check token balance of provider
        uint256 balance3 = rewardToken.balanceOf(provider);
        emit log_named_uint("test token balance of provider", balance3);

        // check appproval amounts for permit2
        uint256 allowance = rewardToken.allowance(requester, PERMIT2);
        emit log_named_uint("permit2 requester allowance", allowance);
        uint256 allowance2 = rewardToken.allowance(provider, PERMIT2);
        emit log_named_uint("permit2 provider allowance", allowance2);

        // check appproval amounts for bombetta
        uint256 allowance3 = rewardToken.allowance(requester, bombetta);
        emit log_named_uint("bombetta requester allowance", allowance3);
        // check appproval amounts for porchetta
        uint256 allowance4 = rewardToken.allowance(provider, bombetta);
        emit log_named_uint("bombetta provider allowance", allowance4);

        // check appproval amounts for porchetta
        uint256 allowance5 = rewardToken.allowance(requester, PERMIT2);
        emit log_named_uint("porchetta requester allowance", allowance5);
        // check appproval amounts for porchetta
        uint256 allowance6 = rewardToken.allowance(provider, PERMIT2);
        emit log_named_uint("porchetta provider allowance", allowance6);

        // check token balances of market contracts
        emit log_named_uint("bombetta token balance", rewardToken.balanceOf(bombetta));
        emit log_named_uint("porchetta token balance", rewardToken.balanceOf(porchetta));

        // check eth balances of the market contracts
        emit log_named_uint("bombetta eth balance", bombetta.balance);
        emit log_named_uint("porchetta eth balance", porchetta.balance);

        vm.stopBroadcast();
    }
}