mod internal_memory_ops;
pub use internal_memory_ops::PagedMemory;
use internal_memory_ops::Transaction;

#[cfg(feature = "naive")]
pub use internal_memory_ops::NaiveMemory;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd, Copy)]
#[repr(transparent)]
pub struct TransactionId(u32);

pub trait Memory: internal_memory_ops::InternalMemoryOps {
    fn read(&self, addr: usize, size: usize) -> Vec<u8>;
    fn read_transaction_ids(&self, addr: usize, size: usize) -> Vec<TransactionId>;
    fn current_transaction_id(&self) -> usize;

    fn next_transaction(&mut self) -> Result<(), ()> {
        let current_idx = self.current_transaction_id();
        let Some(original_transaction) = self.get_mut_transaction(current_idx) else {
            return Err(());
        };
        let transaction_idx = TransactionId((current_idx + 1) as u32);
        let transaction = std::mem::take(original_transaction);
        self.write_data(transaction.addr, &transaction.data);
        self.write_transaction_ids(
            transaction.addr,
            &vec![transaction_idx; transaction.data.len()],
        );
        let original_transaction = self.get_mut_transaction(current_idx).unwrap();
        let _ = std::mem::replace(original_transaction, transaction);
        self.set_transaction_idx(current_idx + 1);
        Ok(())
    }

    fn previous_transaction(&mut self) -> Result<(), ()> {
        let current_idx = self.current_transaction_id();
        if current_idx == 0 {
            return Err(());
        }
        let Some(original_transaction) = self.get_mut_transaction(current_idx - 1) else {
            return Err(());
        };
        let transaction = std::mem::take(original_transaction);
        self.write_data(transaction.addr, &transaction.old_data);
        self.write_transaction_ids(transaction.addr, &transaction.old_ids);
        let original_transaction = self.get_mut_transaction(current_idx - 1).unwrap();
        let _ = std::mem::replace(original_transaction, transaction);
        self.set_transaction_idx(current_idx - 1);
        Ok(())
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
        if addr + data.len() >= self.address_space_size() {
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
        debug_assert!(result.is_ok());
        Ok(())
    }

    fn move_to_transaction(&mut self, idx: TransactionId) -> Result<(), ()> {
        let id = idx.0 as usize;
        if id >= self.transaction_vec_len() {
            Err(())
        } else if id == self.current_transaction_id() {
            Ok(())
        } else if id < self.current_transaction_id() {
            while id < self.current_transaction_id() {
                let result = self.previous_transaction();
                debug_assert!(result.is_ok());
            }
            Ok(())
        } else if id > self.current_transaction_id() {
            while id > self.current_transaction_id() {
                let result = self.next_transaction();
                debug_assert!(result.is_ok());
            }
            Ok(())
        } else {
            unreachable!();
        }
    }
}

#[cfg(all(feature = "naive", test))]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn apply_transaction() {
        let mut memory = NaiveMemory::<4, 4, 16>::new(0xab);
        let data1 = vec![0, 1, 2, 3, 4];
        memory.add_transaction(0x1, data1.clone(), 0x0).unwrap();
        assert_eq!(memory.read(0x1, data1.len()), data1);

        let data2 = vec![4, 3, 2, 1];
        memory.add_transaction(0x3, data2.clone(), 0x0).unwrap();
        let result = memory.read(0x0, 8);
        let expected_result = vec![0xab, 0, 1, 4, 3, 2, 1, 0xab];
        assert_eq!(result, expected_result);
        let result_tr = memory.read_transaction_ids(0x0, 8);
        assert_eq!(result_tr.len(), 8);
        let expected_result_tr = vec![
            TransactionId(0),
            TransactionId(1),
            TransactionId(1),
            TransactionId(2),
            TransactionId(2),
            TransactionId(2),
            TransactionId(2),
            TransactionId(0),
        ];
        assert_eq!(result_tr, expected_result_tr);
    }

    #[test]
    fn revert_transaction() {
        let mut memory = NaiveMemory::<4, 4, 16>::new(0xab);
        let data1 = vec![0, 1, 2, 3, 4];
        memory.add_transaction(0x1, data1.clone(), 0x0).unwrap();
        assert_eq!(memory.read(0x1, data1.len()), data1);

        let data2 = vec![4, 3, 2, 1];
        memory.add_transaction(0x3, data2.clone(), 0x0).unwrap();

        assert!(memory.previous_transaction().is_ok());

        let result = memory.read(0x0, 8);
        let expected_result = vec![0xab, 0, 1, 2, 3, 4, 0xab, 0xab];
        assert_eq!(result, expected_result);
        let result_tr = memory.read_transaction_ids(0x0, 8);
        assert_eq!(result_tr.len(), 8);
        let expected_result_tr = vec![
            TransactionId(0),
            TransactionId(1),
            TransactionId(1),
            TransactionId(1),
            TransactionId(1),
            TransactionId(1),
            TransactionId(0),
            TransactionId(0),
        ];
        assert_eq!(result_tr, expected_result_tr);

        assert!(memory.previous_transaction().is_ok());

        let result = memory.read(0x0, 8);
        let expected_result = vec![0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab];
        assert_eq!(result, expected_result);
        let result_tr = memory.read_transaction_ids(0x0, 8);
        assert_eq!(result_tr.len(), 8);
        let expected_result_tr = vec![
            TransactionId(0),
            TransactionId(0),
            TransactionId(0),
            TransactionId(0),
            TransactionId(0),
            TransactionId(0),
            TransactionId(0),
            TransactionId(0),
        ];
        assert_eq!(result_tr, expected_result_tr);
    }
}
