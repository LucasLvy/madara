use std::collections::BTreeMap;
use std::vec;

use anyhow::{anyhow, Result};
use base64::engine::general_purpose;
use base64::Engine;
use cairo_vm::types::program::Program;
use flate2::read::GzDecoder;
use mp_starknet::execution::types::{ContractClassWrapper, EntryPointTypeWrapper, EntryPointWrapper, MaxEntryPoints};
use mp_starknet::transaction::types::{DeclareTransaction, DeployAccountTransaction, InvokeTransaction, Transaction};
use sp_core::U256;
use sp_runtime::{BoundedBTreeMap, BoundedVec};
use starknet_api::api_core::{calculate_contract_address, ClassHash, ContractAddress as StarknetContractAddress};
use starknet_api::hash::StarkFelt;
use starknet_api::transaction::{Calldata, ContractAddressSalt};
use starknet_core::types::{
    BroadcastedDeclareTransaction, BroadcastedDeployAccountTransaction, BroadcastedInvokeTransaction,
    BroadcastedTransaction, ContractClass, EntryPointsByType, FieldElement, FlattenedSierraClass,
    LegacyEntryPointsByType, StarknetError,
};

/// Returns a `ContractClass` from a `ContractClassWrapper`
// TODO: see https://github.com/keep-starknet-strange/madara/issues/363
pub fn to_rpc_contract_class(_contract_class_wrapped: ContractClassWrapper) -> Result<ContractClass> {
    let entry_points_by_type = EntryPointsByType { constructor: vec![], external: vec![], l1_handler: vec![] };
    let default = FlattenedSierraClass {
        sierra_program: vec![FieldElement::from_dec_str("0").unwrap()],
        contract_class_version: String::from("version"),
        entry_points_by_type,
        abi: String::from(""),
    };
    Ok(ContractClass::Sierra(default))
}

/// Returns a base64 encoded and compressed string of the input bytes
pub(crate) fn _compress_and_encode_base64(data: &[u8]) -> Result<String> {
    let data_compressed = _compress(data)?;
    Ok(_encode_base64(&data_compressed))
}

/// Returns a compressed vector of bytes
pub(crate) fn _compress(data: &[u8]) -> Result<Vec<u8>> {
    let mut gzip_encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    serde_json::to_writer(&mut gzip_encoder, data)?;
    Ok(gzip_encoder.finish()?)
}

/// Returns a base64 encoded string of the input bytes
pub(crate) fn _encode_base64(data: &[u8]) -> String {
    general_purpose::STANDARD.encode(data)
}

pub fn to_tx(request: BroadcastedTransaction, chain_id: &str) -> Result<Transaction> {
    match request {
        BroadcastedTransaction::Invoke(invoke_tx) => to_invoke_tx(invoke_tx).map(|inner| inner.from_invoke(chain_id)),
        BroadcastedTransaction::Declare(_) => Err(StarknetError::FailedToReceiveTransaction.into()), /* TODO: add support once #341 is supported */
        BroadcastedTransaction::DeployAccount(deploy_account_tx) => {
            to_deploy_account_tx(deploy_account_tx).map(|inner| inner.from_deploy(chain_id))
        }
    }
}

pub fn to_invoke_tx(tx: BroadcastedInvokeTransaction) -> Result<InvokeTransaction> {
    match tx {
        BroadcastedInvokeTransaction::V0(_) => Err(StarknetError::FailedToReceiveTransaction.into()),
        BroadcastedInvokeTransaction::V1(invoke_tx_v1) => Ok(InvokeTransaction {
            version: 1_u8,
            signature: BoundedVec::try_from(
                invoke_tx_v1.signature.iter().map(|x| U256::from_big_endian(&x.to_bytes_be())).collect::<Vec<U256>>(),
            )
            .map_err(|e| anyhow!("failed to convert signature: {:?}", e))?,

            sender_address: U256::from_big_endian(&invoke_tx_v1.sender_address.to_bytes_be()),
            nonce: U256::from_big_endian(&invoke_tx_v1.nonce.to_bytes_be()),
            calldata: BoundedVec::try_from(
                invoke_tx_v1.calldata.iter().map(|x| U256::from_big_endian(&x.to_bytes_be())).collect::<Vec<U256>>(),
            )
            .map_err(|e| anyhow!("failed to convert calldata: {:?}", e))?,
            max_fee: U256::from_big_endian(&invoke_tx_v1.max_fee.to_bytes_be()),
        }),
    }
}

pub fn to_deploy_account_tx(tx: BroadcastedDeployAccountTransaction) -> Result<DeployAccountTransaction> {
    let contract_address_salt = tx.contract_address_salt.to_bytes_be();

    let account_class_hash = tx.class_hash;

    let calldata =
        tx.constructor_calldata.iter().filter_map(|f| StarkFelt::new(f.to_bytes_be()).ok()).collect::<Vec<_>>();

    let signature = tx
        .signature
        .iter()
        .map(|f| U256::from_big_endian(&f.to_bytes_be()))
        .collect::<Vec<U256>>()
        .try_into()
        .map_err(|_| anyhow!("failed to bound signatures Vec<H256> by MaxArraySize"))?;

    let sender_address = U256::from_big_endian(
        &calculate_contract_address(
            ContractAddressSalt(StarkFelt(contract_address_salt)),
            ClassHash(StarkFelt(account_class_hash.to_bytes_be())),
            &Calldata(calldata.into()),
            StarknetContractAddress::default(),
        )
        .map_err(|e| anyhow!("Failed to calculate contract address: {e}"))?
        .0
        .0
        .0,
    );

    let calldata = tx
        .constructor_calldata
        .iter()
        .map(|f| U256::from_big_endian(&f.to_bytes_be()))
        .collect::<Vec<U256>>()
        .try_into()
        .map_err(|_| anyhow!("failed to bound calldata Vec<U256> by MaxArraySize"))?;

    let nonce = U256::from_big_endian(&tx.nonce.to_bytes_be());
    let max_fee = U256::from_big_endian(&tx.max_fee.to_bytes_be());

    Ok(DeployAccountTransaction {
        version: 1_u8,
        sender_address,
        calldata,
        salt: U256::from(contract_address_salt),
        signature,
        account_class_hash: U256::from_big_endian(&account_class_hash.to_bytes_be()),
        nonce,
        max_fee,
    })
}

pub fn to_declare_tx(tx: BroadcastedDeclareTransaction) -> Result<DeclareTransaction> {
    match tx {
        BroadcastedDeclareTransaction::V1(declare_tx_v1) => {
            let signature = declare_tx_v1
                .signature
                .iter()
                .map(|f| U256::from_big_endian(&f.to_bytes_be()))
                .collect::<Vec<U256>>()
                .try_into()
                .map_err(|_| anyhow!("failed to bound signatures Vec<H256> by MaxArraySize"))?;

            // Create a GzipDecoder to decompress the bytes
            let mut gz = GzDecoder::new(&declare_tx_v1.contract_class.program[..]);

            // Read the decompressed bytes into a Vec<u8>
            let mut decompressed_bytes = Vec::new();
            std::io::Read::read_to_end(&mut gz, &mut decompressed_bytes)
                .map_err(|_| anyhow!("Failed to decompress the contract class program"))?;

            // Deserialize it then
            let program: Program = Program::from_bytes(&decompressed_bytes, None)
                .map_err(|_| anyhow!("Failed to deserialize the contract class program"))?;

            Ok(DeclareTransaction {
                version: 1_u8,
                sender_address: U256::from_big_endian(&declare_tx_v1.sender_address.to_bytes_be()),
                nonce: U256::from_big_endian(&declare_tx_v1.nonce.to_bytes_be()),
                max_fee: U256::from_big_endian(&declare_tx_v1.max_fee.to_bytes_be()),
                signature,
                contract_class: ContractClassWrapper {
                    program: program.try_into().map_err(|_| anyhow!("Failed to convert program to program wrapper"))?,
                    entry_points_by_type: BoundedBTreeMap::try_from(_to_btree_map_entrypoints(
                        declare_tx_v1.contract_class.entry_points_by_type.clone(),
                    ))
                    .unwrap(),
                },
                compiled_class_hash: U256::zero(), // TODO: compute class hash
            })
        }
        BroadcastedDeclareTransaction::V2(_) => Err(StarknetError::FailedToReceiveTransaction.into()),
    }
}

/// Returns a hash map of entry point types to entrypoint from deprecated entry point by type
fn _to_btree_map_entrypoints(
    entries: LegacyEntryPointsByType,
) -> BTreeMap<EntryPointTypeWrapper, BoundedVec<EntryPointWrapper, MaxEntryPoints>> {
    let mut entry_points_by_type: BTreeMap<EntryPointTypeWrapper, BoundedVec<EntryPointWrapper, MaxEntryPoints>> =
        BTreeMap::new();
    // We can unwrap safely as we already checked the length of the vectors
    entry_points_by_type.insert(
        EntryPointTypeWrapper::Constructor,
        BoundedVec::try_from(
            entries.constructor.iter().map(|e| EntryPointWrapper::from(e.clone())).collect::<Vec<_>>(),
        )
        .unwrap(),
    );
    entry_points_by_type.insert(
        EntryPointTypeWrapper::External,
        BoundedVec::try_from(entries.external.iter().map(|e| EntryPointWrapper::from(e.clone())).collect::<Vec<_>>())
            .unwrap(),
    );
    entry_points_by_type.insert(
        EntryPointTypeWrapper::L1Handler,
        BoundedVec::try_from(entries.l1_handler.iter().map(|e| EntryPointWrapper::from(e.clone())).collect::<Vec<_>>())
            .unwrap(),
    );
    entry_points_by_type
}
