use std::num::NonZeroU32;

mod paged_memory;

pub use paged_memory::PagedMemory;

#[cfg(feature = "naive")]
mod naive_memory;
#[cfg(feature = "naive")]
pub use naive_memory::NaiveMemory;

pub trait InternalMemoryOps {
    fn transaction_vec_len(&self) -> usize;
    fn transaction_vec_push(&mut self, transaction: Transaction);
    fn get_mut_transaction(&mut self, idx: usize) -> Option<&mut Transaction>;
    fn set_transaction_idx(&mut self, idx: usize);
    fn write_data(&mut self, addr: usize, data: &[u8]);
    fn write_transaction_ids(&mut self, addr: usize, transaction_ids: &[Option<NonZeroU32>]);
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Transaction {
    pub addr: usize,
    pub data: Vec<u8>,
    pub old_data: Vec<u8>,
    pub old_ids: Vec<Option<NonZeroU32>>,
    pub code_location: usize,
}
