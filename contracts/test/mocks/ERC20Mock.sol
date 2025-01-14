// SPDX-License-Identifier: AGPL-3.0-only
pragma solidity ^0.8.15;

import {ERC20} from "solmate/tokens/ERC20.sol";

contract ERC20Mock is ERC20 {
    address[] public admin_addresses;

    constructor(string memory name_, string memory symbol_, uint8 decimals_) ERC20(name_, symbol_, decimals_) {
        admin_addresses.push(msg.sender);
    }

    modifier onlyAdmin() {
        require(isAdmin(msg.sender), "Caller is not an admin");
        _;
    }

    function isAdmin(address account) public view returns (bool) {
        for (uint256 i = 0; i < admin_addresses.length; i++) {
            if (admin_addresses[i] == account) {
                return true;
            }
        }
        return false;
    }

    function addAdmin(address newAdmin) external onlyAdmin {
        require(!isAdmin(newAdmin), "Address is already an admin");
        admin_addresses.push(newAdmin);
    }

    function mint(address to, uint256 amount) external onlyAdmin {
        _mint(to, amount);
    }

    function transfer(address to, uint256 amount) public override returns (bool) {
        return super.transfer(to, amount);
    }

    function transferFrom(address from, address to, uint256 amount) public override returns (bool) {
        return super.transferFrom(from, to, amount);
    }
}
