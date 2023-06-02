use frame_support::{assert_ok, bounded_vec};
use lazy_static::lazy_static;
use mp_starknet::crypto::commitment::calculate_invoke_tx_hash;
use mp_starknet::execution::types::ContractClassWrapper;
use mp_starknet::transaction::types::{EventWrapper, InvokeTransaction};
use sp_core::U256;

use super::mock::*;
use super::utils::get_contract_class_wrapper;
use crate::tests::constants::TOKEN_CONTRACT_CLASS_HASH;
use crate::Event;

lazy_static! {
    static ref ERC20_CONTRACT_CLASS: ContractClassWrapper = get_contract_class_wrapper("erc20/erc20.json");
}

#[test]
fn given_erc20_transfer_when_invoke_then_it_works() {
    new_test_ext().execute_with(|| {
        System::set_block_number(0);
        run_to_block(1);
        let origin = RuntimeOrigin::none();
        let sender_account = get_account_address(AccountType::NoValidate);
        // ERC20 is already declared for the fees.
        // Deploy ERC20 contract
        let deploy_transaction = InvokeTransaction {
            version: 1,
            sender_address: sender_account,
            calldata: bounded_vec![
                sender_account, // Simple contract address
                U256::from_str_radix("0x02730079d734ee55315f4f141eaed376bddd8c2133523d223a344c5604e0f7f8", 16)
                    .unwrap(), // deploy_contract selector
                U256::from_str_radix("0x9", 16).unwrap(), // Calldata len
                U256::from_str_radix(TOKEN_CONTRACT_CLASS_HASH, 16).unwrap(), // Class hash
                U256::one(), // Contract address salt
                U256::from_str_radix("0x6", 16).unwrap(), // Constructor_calldata_len
                U256::from_str_radix("0xA", 16).unwrap(), // Name
                U256::from_str_radix("0x1", 16).unwrap(), // Symbol
                U256::from_str_radix("0x2", 16).unwrap(), // Decimals
                U256::from_str_radix("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", 16).unwrap(), // Initial supply low
                U256::from_str_radix("0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF", 16).unwrap(), // Initial supply high
                sender_account  // recipient
            ],
            nonce: U256::zero(),
            max_fee: U256::from(u128::MAX),
            signature: bounded_vec!(),
        };
        let chain_id = Starknet::chain_id_str();
        let transaction_hash = calculate_invoke_tx_hash(deploy_transaction.clone(), &chain_id);

        let expected_erc20_address =
            U256::from_str_radix("0x00dc58c1280862c95964106ef9eba5d9ed8c0c16d05883093e4540f22b829dff", 16).unwrap();

        assert_ok!(Starknet::invoke(origin.clone(), deploy_transaction));
        let events = System::events();
        // Check transaction event (deployment)
        pretty_assertions::assert_eq!(
            Event::<MockRuntime>::StarknetEvent(EventWrapper {
                keys: bounded_vec![
                    U256::from_str_radix("0x026b160f10156dea0639bec90696772c640b9706a47f5b8c52ea1abe5858b34d", 16)
                        .unwrap()
                ],
                data: bounded_vec!(
                    expected_erc20_address, // Contract address
                    U256::zero(),   /* Deployer (always 0 with this
                                             * account contract) */
                    U256::from_str_radix(TOKEN_CONTRACT_CLASS_HASH, 16).unwrap(), // Class hash
                    U256::from_str_radix("0x0000000000000000000000000000000000000000000000000000000000000006", 16)
                        .unwrap(), // Constructor calldata len
                    U256::from_str_radix("0x000000000000000000000000000000000000000000000000000000000000000a", 16)
                        .unwrap(), // Name
                    U256::from_str_radix("0x0000000000000000000000000000000000000000000000000000000000000001", 16)
                        .unwrap(), // Symbol
                    U256::from_str_radix("0x0000000000000000000000000000000000000000000000000000000000000002", 16)
                        .unwrap(), // Decimals
                    U256::from_str_radix("0x000000000000000000000000000000000fffffffffffffffffffffffffffffff", 16)
                        .unwrap(), // Initial supply low
                    U256::from_str_radix("0x000000000000000000000000000000000fffffffffffffffffffffffffffffff", 16)
                        .unwrap(), // Initial supply high
                    U256::from_str_radix("0x01a3339ec92ac1061e3e0f8e704106286c642eaf302e94a582e5f95ef5e6b4d0", 16)
                        .unwrap(), // Recipient
                    U256::from_str_radix("0x0000000000000000000000000000000000000000000000000000000000000001", 16)
                        .unwrap(), // Salt
                ),
                from_address: sender_account,
                transaction_hash
            }),
            events[events.len() - 2].event.clone().try_into().unwrap(),
        );
        let expected_fee_transfer_event = Event::StarknetEvent(EventWrapper {
            keys: bounded_vec![
                U256::from_str_radix("0x0099cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9", 16)
                    .unwrap()
            ],
            data: bounded_vec!(
                sender_account, // From
                U256::from_str_radix("0x0000000000000000000000000000000000000000000000000000000000000002", 16)
                    .unwrap(), // Sequencer address
                U256::from_str_radix("0x000000000000000000000000000000000000000000000000000000000002b660", 16)
                    .unwrap(), // Amount low
                U256::zero(), // Amount high
            ),
            from_address: Starknet::fee_token_address(),
            transaction_hash,
        });
        // Check fee transfer event
        pretty_assertions::assert_eq!(
            expected_fee_transfer_event,
            events.last().unwrap().event.clone().try_into().unwrap()
        );
        // TODO: use dynamic values to craft invoke transaction
        // Transfer some token
        let transfer_transaction = InvokeTransaction {
            version: 1,
            sender_address: sender_account,
            calldata: bounded_vec![
                expected_erc20_address, // Token address
                U256::from_str_radix("0x0083afd3f4caedc6eebf44246fe54e38c95e3179a5ec9ea81740eca5b482d12e", 16)
                    .unwrap(), // transfer selector
                U256::from(3),  // Calldata len
                U256::from(16u128), // recipient
                U256::from(15u128), // initial supply low
                U256::zero(),   // initial supply high
            ],
            nonce: U256::one(),
            max_fee: U256::from(u128::MAX),
            signature: bounded_vec!(),
        };
        let chain_id = Starknet::chain_id_str();
        let transaction_hash = calculate_invoke_tx_hash(transfer_transaction.clone(), &chain_id);

        // Also asserts that the deployment has been saved.
        assert_ok!(Starknet::invoke(origin, transfer_transaction));
        pretty_assertions::assert_eq!(
            Starknet::storage((
                expected_erc20_address,
                Into::<U256>::into(
                    U256::from_str_radix("03701645da930cd7f63318f7f118a9134e72d64ab73c72ece81cae2bd5fb403f", 16)
                        .unwrap()
                )
            )),
            U256::from_str_radix("ffffffffffffffffffffffffffffff0", 16).unwrap()
        );
        pretty_assertions::assert_eq!(
            Starknet::storage((
                expected_erc20_address,
                Into::<U256>::into(
                    U256::from_str_radix("03701645da930cd7f63318f7f118a9134e72d64ab73c72ece81cae2bd5fb4040", 16)
                        .unwrap()
                )
            )),
            U256::from_str_radix("fffffffffffffffffffffffffffffff", 16).unwrap()
        );

        pretty_assertions::assert_eq!(
            Starknet::storage((
                expected_erc20_address,
                Into::<U256>::into(
                    U256::from_str_radix("0x011cb0dc747a73020cbd50eac7460edfaa7d67b0e05823b882b05c3f33b1c73e", 16)
                        .unwrap()
                )
            )),
            U256::from(15u128)
        );
        pretty_assertions::assert_eq!(
            Starknet::storage((
                expected_erc20_address,
                Into::<U256>::into(
                    U256::from_str_radix("0x011cb0dc747a73020cbd50eac7460edfaa7d67b0e05823b882b05c3f33b1c73f", 16)
                        .unwrap()
                )
            )),
            U256::zero()
        );

        let events = System::events();
        // Check regular event.
        let expected_event = Event::StarknetEvent(EventWrapper {
            keys: bounded_vec![
                U256::from_str_radix("0x0099cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9", 16)
                    .unwrap()
            ],
            data: bounded_vec!(
                U256::from_str_radix("0x01a3339ec92ac1061e3e0f8e704106286c642eaf302e94a582e5f95ef5e6b4d0", 16)
                    .unwrap(), // From
                U256::from_str_radix("0x10", 16).unwrap(), // To
                U256::from_str_radix("0xF", 16).unwrap(),  // Amount low
                U256::zero(),                         // Amount high
            ),
            from_address: U256::from_str_radix(
                "0x00dc58c1280862c95964106ef9eba5d9ed8c0c16d05883093e4540f22b829dff", 16
            )
            .unwrap(),
            transaction_hash,
        });

        pretty_assertions::assert_eq!(expected_event, events[events.len() - 2].event.clone().try_into().unwrap());
        // Check fee transfer.
        let expected_fee_transfer_event = Event::StarknetEvent(EventWrapper {
            keys: bounded_vec![
                U256::from_str_radix("0x0099cd8bde557814842a3121e8ddfd433a539b8c9f14bf31ebf108d12e6196e9", 16)
                    .unwrap()
            ],
            data: bounded_vec!(
                sender_account,                                  // From
                U256::from_str_radix("0x2", 16).unwrap(),     // Sequencer address
                U256::from_str_radix("0x1e618", 16).unwrap(), // Amount low
                U256::zero(),                            // Amount high
            ),
            from_address: Starknet::fee_token_address(),
            transaction_hash,
        });
        pretty_assertions::assert_eq!(
            expected_fee_transfer_event,
            events.last().unwrap().event.clone().try_into().unwrap()
        );
    })
}
