use frame_support::{assert_ok, bounded_vec};
use mp_starknet::transaction::types::InvokeTransaction;
use sp_core::{ConstU32, U256};
use sp_runtime::BoundedVec;

use super::constants::TOKEN_CONTRACT_CLASS_HASH;
use super::mock::*;

#[test]
fn given_call_contract_call_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(0);
        run_to_block(1);

        let origin = RuntimeOrigin::none();
        let sender_account = get_account_address(AccountType::NoValidate);

        // Deploy ERC20 Contract, as it is already declared in fixtures
        // Deploy ERC20 contract
        let constructor_calldata: BoundedVec<U256, ConstU32<{ u32::MAX }>> = bounded_vec![
            sender_account, // Simple contract address
            U256::from_str_radix("0x02730079d734ee55315f4f141eaed376bddd8c2133523d223a344c5604e0f7f8", 16)
                .unwrap(), // deploy_contract selector
            U256::from_str_radix("0x0000000000000000000000000000000000000000000000000000000000000009", 16)
                .unwrap(), // Calldata len
            U256::from_str_radix(TOKEN_CONTRACT_CLASS_HASH, 16).unwrap(), // Class hash
            U256::one(), // Contract address salt
            U256::from_str_radix("0x6", 16).unwrap(), // Constructor_calldata_len
            U256::from_str_radix("0xA", 16).unwrap(), // Name
            U256::from_str_radix("0x1", 16).unwrap(), // Symbol
            U256::from_str_radix("0x2", 16).unwrap(), // Decimals
            U256::from_str_radix("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", 16).unwrap(), // Initial supply low
            U256::from_str_radix("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", 16).unwrap(), // Initial supply high
            sender_account  // recipient
        ];

        let deploy_transaction = InvokeTransaction {
            version: 1,
            sender_address: sender_account,
            signature: bounded_vec!(),
            nonce: U256::zero(),
            calldata: constructor_calldata,
            max_fee: U256::from(u128::MAX),
        };

        assert_ok!(Starknet::invoke(origin, deploy_transaction));

        let expected_erc20_address =
            U256::from_str_radix("00dc58c1280862c95964106ef9eba5d9ed8c0c16d05883093e4540f22b829dff", 16).unwrap();

        // Call balanceOf
        let balance_of_selector =
            U256::from_str_radix("0x02e4263afad30923c891518314c3c95dbe830a16874e8abc5777a9a20b54c76e", 16).unwrap();
        let calldata = bounded_vec![
            sender_account // owner address
        ];
        let res = Starknet::call_contract(expected_erc20_address, balance_of_selector, calldata);
        assert_ok!(res.clone());
        pretty_assertions::assert_eq!(
            res.unwrap(),
            vec![
                U256::from_str_radix("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", 16).unwrap(),
                U256::from_str_radix("FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", 16).unwrap()
            ]
        );

        // Call symbol
        let symbol_selector =
            U256::from_str_radix("0x0216b05c387bab9ac31918a3e61672f4618601f3c598a2f3f2710f37053e1ea4", 16).unwrap();
        let calldata = bounded_vec![];
        let res = Starknet::call_contract(expected_erc20_address, symbol_selector, calldata);
        assert_ok!(res.clone());
        pretty_assertions::assert_eq!(res.unwrap(), vec![U256::from_str_radix("0x01", 16).unwrap()]);

        // Call name
        let name_selector =
            U256::from_str_radix("0x0361458367e696363fbcc70777d07ebbd2394e89fd0adcaf147faccd1d294d60", 16).unwrap();
        let calldata = bounded_vec![];
        let res = Starknet::call_contract(expected_erc20_address, name_selector, calldata);
        assert_ok!(res.clone());
        pretty_assertions::assert_eq!(res.unwrap(), vec![U256::from_str_radix("0x0A", 16).unwrap()]);

        // Call decimals
        let decimals_selector =
            U256::from_str_radix("0x004c4fb1ab068f6039d5780c68dd0fa2f8742cceb3426d19667778ca7f3518a9", 16).unwrap();
        let calldata = bounded_vec![];
        let res = Starknet::call_contract(expected_erc20_address, decimals_selector, calldata);
        assert_ok!(res.clone());
        pretty_assertions::assert_eq!(res.unwrap(), vec![U256::from_str_radix("0x02", 16).unwrap()]);
    });
}
