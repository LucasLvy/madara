use sp_core::U256;
use starknet_ff::FieldElement;

pub struct RpcEventFilter {
    pub from_block: u64,
    pub to_block: u64,
    pub from_address: Option<U256>,
    pub keys: Vec<Vec<FieldElement>>,
    pub chunk_size: u64,
    pub continuation_token: usize,
}
