use std::num::NonZeroU32;

mod internal_memory_ops;
pub use internal_memory_ops::PagedMemory;
use internal_memory_ops::Transaction;

#[cfg(feature = "naive")]
pub use internal_memory_ops::NaiveMemory;

pub trait Memory: internal_memory_ops::InternalMemoryOps {
    fn read(&self, addr: usize, size: usize) -> Vec<u8>;
    fn read_transaction_ids(&self, addr: usize, size: usize) -> Vec<Option<NonZeroU32>>;
    fn current_transaction_id(&self) -> usize;

    fn next_transaction(&mut self) -> Option<()> {
        let current_idx = self.current_transaction_id();
        let Some(original_transaction) = self.get_mut_transaction(current_idx) else {
            return None;
        };
        let transaction_idx = NonZeroU32::new((current_idx + 1) as u32);
        let transaction = std::mem::take(original_transaction);
        self.write_data(transaction.addr, &transaction.data);
        self.write_transaction_ids(
            transaction.addr,
            &vec![transaction_idx; transaction.data.len()],
        );
        let original_transaction = self.get_mut_transaction(current_idx).unwrap();
        let _ = std::mem::replace(original_transaction, transaction);
        self.set_transaction_idx(current_idx + 1);
        Some(())
    }

    fn previous_transaction(&mut self) -> Option<()> {
        let current_idx = self.current_transaction_id();
        if current_idx == 0 {
            return None;
        }
        let Some(original_transaction) = self.get_mut_transaction(current_idx - 1) else {
            return None;
        };
        let transaction = std::mem::take(original_transaction);
        self.write_data(transaction.addr, &transaction.old_data);
        self.write_transaction_ids(transaction.addr, &transaction.old_ids);
        let original_transaction = self.get_mut_transaction(current_idx - 1).unwrap();
        let _ = std::mem::replace(original_transaction, transaction);
        self.set_transaction_idx(current_idx - 1);
        Some(())
    }

    fn add_transaction(
        &mut self,
        addr: usize,
        data: Vec<u8>,
        code_location: usize,
    ) -> Result<(), ()> {
        if self.transaction_vec_len() != self.current_transaction_id() {
            return Err(());
        }
        let old_data = self.read(addr, data.len());
        let old_ids = self.read_transaction_ids(addr, data.len());
        let transaction = Transaction {
            addr,
            data,
            old_ids,
            old_data,
            code_location,
        };
        self.transaction_vec_push(transaction);
        let result = self.next_transaction();
        debug_assert!(result.is_some());
        Ok(())
    }
}
