use std::num::NonZeroU32;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct Page<const SIZE: usize> {
    data: [u8; SIZE],
    transaction_ids: [Option<NonZeroU32>; SIZE],
}
impl<const SIZE: usize> Page<SIZE> {
    pub fn new(default_value: u8) -> Self {
        Self {
            data: [default_value; SIZE],
            transaction_ids: std::array::from_fn(|_| None),
        }
    }
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct Transaction {
    addr: usize,
    data: Vec<u8>,
    old_data: Vec<u8>,
    old_ids: Vec<Option<NonZeroU32>>,
    code_location: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct Memory<const NUM_PAGES: usize, const PAGE_SIZE: usize> {
    default_value: u8,
    memory: [Option<Box<Page<PAGE_SIZE>>>; NUM_PAGES],
    transactions: Vec<Transaction>,
    transaction_idx: usize,
}

impl<const NUM_PAGES: usize, const PAGE_SIZE: usize> Memory<NUM_PAGES, PAGE_SIZE> {
    const COMPTIME_SIZE_CHECK_PAGE: () = assert!(2_usize.pow(PAGE_SIZE.ilog2()) == PAGE_SIZE);
    const COMPTIME_SIZE_CHECK_SPACE: () = assert!(2_usize.pow(NUM_PAGES.ilog2()) == NUM_PAGES);

    pub fn new(default_value: u8) -> Self {
        let _: () = Self::COMPTIME_SIZE_CHECK_PAGE;
        let _: () = Self::COMPTIME_SIZE_CHECK_SPACE;
        Self {
            default_value,
            memory: std::array::from_fn(|_| None),
            transactions: Vec::new(),
            transaction_idx: 0,
        }
    }

    fn read_page_data(
        &self,
        idx: usize,
        in_page_start_addr: usize,
        in_page_end_addr: usize,
    ) -> Vec<u8> {
        if let Some(page_data) = self.memory[idx].as_ref().map(|page| &page.data) {
            page_data[in_page_start_addr..=in_page_end_addr].to_vec()
        } else {
            vec![self.default_value; in_page_end_addr + 1 - in_page_start_addr]
        }
    }

    fn read_page_transaction_ids(
        &self,
        idx: usize,
        in_page_start_addr: usize,
        in_page_end_addr: usize,
    ) -> Vec<Option<NonZeroU32>> {
        if let Some(page_data) = self.memory[idx].as_ref().map(|page| &page.transaction_ids) {
            page_data[in_page_start_addr..=in_page_end_addr].to_vec()
        } else {
            vec![NonZeroU32::new(0); in_page_end_addr + 1 - in_page_start_addr]
        }
    }

    pub fn read(&self, addr: usize, size: usize) -> Vec<u8> {
        assert!(size > 0);
        let in_page_addr_mask = (1 << (PAGE_SIZE.ilog2())) - 1;
        let page_addr_shift = PAGE_SIZE.ilog2();

        let start_addr = addr;
        let end_addr = addr + size - 1;
        let start_page_addr = start_addr >> page_addr_shift;
        let end_page_addr = end_addr >> page_addr_shift;
        let in_page_start_addr = start_addr & in_page_addr_mask;
        let in_page_end_addr = end_addr & in_page_addr_mask;

        let mut data;
        if start_page_addr == end_page_addr {
            data = self.read_page_data(start_page_addr, in_page_start_addr, in_page_end_addr);
        } else {
            data = self.read_page_data(start_page_addr, in_page_start_addr, PAGE_SIZE - 1);
            for page_idx in start_page_addr + 1..end_page_addr {
                data.extend(self.read_page_data(page_idx, 0, PAGE_SIZE - 1));
            }
            data.extend(self.read_page_data(end_page_addr, 0, in_page_end_addr));
        }
        data
    }

    fn read_transaction_ids(&self, addr: usize, size: usize) -> Vec<Option<NonZeroU32>> {
        assert!(size > 0);
        let in_page_addr_mask = (1 << (PAGE_SIZE.ilog2())) - 1;
        let page_addr_shift = PAGE_SIZE.ilog2();

        let start_addr = addr;
        let end_addr = addr + size - 1;
        let start_page_addr = start_addr >> page_addr_shift;
        let end_page_addr = end_addr >> page_addr_shift;
        let in_page_start_addr = start_addr & in_page_addr_mask;
        let in_page_end_addr = end_addr & in_page_addr_mask;

        let mut transaction_ids;
        if start_page_addr == end_page_addr {
            transaction_ids = self.read_page_transaction_ids(
                start_page_addr,
                in_page_start_addr,
                in_page_end_addr,
            );
        } else {
            transaction_ids =
                self.read_page_transaction_ids(start_page_addr, in_page_start_addr, PAGE_SIZE - 1);
            for page_idx in start_page_addr + 1..end_page_addr {
                transaction_ids.extend(self.read_page_transaction_ids(page_idx, 0, PAGE_SIZE - 1));
            }
            transaction_ids.extend(self.read_page_transaction_ids(
                end_page_addr,
                0,
                in_page_end_addr,
            ));
        }
        transaction_ids
    }

    fn write_page_data(&mut self, idx: usize, in_page_start_addr: usize, data: &[u8]) {
        if let Some(page_data) = self.memory[idx].as_mut().map(|page| &mut page.data) {
            for (index, value) in data.iter().enumerate() {
                page_data[in_page_start_addr + index] = *value;
            }
        } else {
            let mut new_page = Page::new(self.default_value);
            for (index, value) in data.iter().enumerate() {
                new_page.data[in_page_start_addr + index] = *value;
            }
            self.memory[idx] = Some(Box::new(new_page));
        }
    }

    fn write_page_transaction_ids(
        &mut self,
        idx: usize,
        in_page_start_addr: usize,
        transaction_ids: &[Option<NonZeroU32>],
    ) {
        if let Some(page_transaction_ids) = self.memory[idx]
            .as_mut()
            .map(|page| &mut page.transaction_ids)
        {
            for (index, value) in transaction_ids.iter().enumerate() {
                page_transaction_ids[in_page_start_addr + index] = *value;
            }
        } else {
            unreachable!("Page should have been already created!");
        }
    }

    fn write_data(&mut self, addr: usize, data: &[u8]) {
        let size = data.len();
        assert!(size > 0);
        let in_page_addr_mask = (1 << (PAGE_SIZE.ilog2())) - 1;
        let page_addr_shift = PAGE_SIZE.ilog2();

        let start_addr = addr;
        let end_addr = addr + size - 1;
        let start_page_addr = start_addr >> page_addr_shift;
        let end_page_addr = end_addr >> page_addr_shift;
        let in_page_start_addr = start_addr & in_page_addr_mask;

        if start_page_addr == end_page_addr {
            self.write_page_data(start_page_addr, in_page_start_addr, &data);
        } else {
            self.write_page_data(
                start_page_addr,
                in_page_start_addr,
                &data[0..PAGE_SIZE - in_page_start_addr],
            );
            let mut offset = PAGE_SIZE - in_page_start_addr;
            for page_idx in start_page_addr + 1..end_page_addr {
                self.write_page_data(page_idx, 0, &data[offset..offset + PAGE_SIZE]);
                offset += PAGE_SIZE;
            }
            self.write_page_data(end_page_addr, 0, &data[offset..]);
        }
    }

    fn write_transaction_ids(&mut self, addr: usize, transaction_ids: &[Option<NonZeroU32>]) {
        let size = transaction_ids.len();
        assert!(size > 0);
        let in_page_addr_mask = (1 << (PAGE_SIZE.ilog2())) - 1;
        let page_addr_shift = PAGE_SIZE.ilog2();

        let start_addr = addr;
        let end_addr = addr + size - 1;
        let start_page_addr = start_addr >> page_addr_shift;
        let end_page_addr = end_addr >> page_addr_shift;
        let in_page_start_addr = start_addr & in_page_addr_mask;

        if start_page_addr == end_page_addr {
            self.write_page_transaction_ids(start_page_addr, in_page_start_addr, &transaction_ids);
        } else {
            self.write_page_transaction_ids(
                start_page_addr,
                in_page_start_addr,
                &transaction_ids[0..PAGE_SIZE - in_page_start_addr],
            );
            let mut offset = PAGE_SIZE - in_page_start_addr;
            for page_idx in start_page_addr + 1..end_page_addr {
                self.write_page_transaction_ids(
                    page_idx,
                    0,
                    &transaction_ids[offset..offset + PAGE_SIZE],
                );
                offset += PAGE_SIZE;
            }
            self.write_page_transaction_ids(end_page_addr, 0, &transaction_ids[offset..]);
        }
    }

    fn next_transaction(&mut self) -> Option<()> {
        let Some(original_transaction) = self.transactions.get_mut(self.transaction_idx) else {
            return None;
        };
        let transaction_idx = NonZeroU32::new((self.transaction_idx + 1) as u32);
        let transaction = std::mem::take(original_transaction);
        self.write_data(transaction.addr, &transaction.data);
        self.write_transaction_ids(
            transaction.addr,
            &vec![transaction_idx; transaction.data.len()],
        );
        let _ = std::mem::replace(&mut self.transactions[self.transaction_idx], transaction);
        self.transaction_idx = self.transaction_idx + 1;
        Some(())
    }

    fn previous_transaction(&mut self) -> Option<()> {
        if self.transaction_idx == 0 {
            return None;
        }
        let Some(original_transaction) = self.transactions.get_mut(self.transaction_idx - 1) else {
            return None;
        };
        let transaction = std::mem::take(original_transaction);
        self.write_data(transaction.addr, &transaction.old_data);
        self.write_transaction_ids(
            transaction.addr,
            &transaction.old_ids,
        );
        let _ = std::mem::replace(&mut self.transactions[self.transaction_idx - 1], transaction);
        self.transaction_idx = self.transaction_idx - 1;
        Some(())
    }

    pub fn add_transaction(
        &mut self,
        addr: usize,
        data: Vec<u8>,
        code_location: usize,
    ) -> Result<(), ()> {
        if self.transactions.len() != self.transaction_idx {
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
        self.transactions.push(transaction);
        let result = self.next_transaction();
        debug_assert!(result.is_some());
        Ok(())
    }

    pub fn current_transaction_id(&self) -> usize {
        self.transaction_idx
    }
}

#[cfg(naive)]
struct NaiveMemory<const NUM_PAGES: usize, const PAGE_SIZE: usize, const SIZE: usize> {
    default_value: u8,
    data: Box<[u8; SIZE]>,
    transaction_ids: Box<[Option<NonZeroU32>; SIZE]>,
    transactions: Vec<Transaction>,
    transaction_idx: usize,
}

#[cfg(naive)]
impl<const NUM_PAGES: usize, const PAGE_SIZE: usize, const SIZE: usize>
    NaiveMemory<NUM_PAGES, PAGE_SIZE, SIZE>
{
    const COMPTIME_SIZE_CHECK_PAGE: () = assert!(2_usize.pow(PAGE_SIZE.ilog2()) == PAGE_SIZE);
    const COMPTIME_SIZE_CHECK_SPACE: () = assert!(2_usize.pow(NUM_PAGES.ilog2()) == NUM_PAGES);
    const COMPTIME_SIZE_CHECK_SIZE: () = assert!(NUM_PAGES * PAGE_SIZE == SIZE);

    pub fn new(default_value: u8) -> Self {
        let _: () = Self::COMPTIME_SIZE_CHECK_PAGE;
        let _: () = Self::COMPTIME_SIZE_CHECK_SPACE;
        let _: () = Self::COMPTIME_SIZE_CHECK_SIZE;
        Self {
            default_value,
            data: Box::new([default_value; SIZE]),
            transaction_ids: Box::new(std::array::from_fn(|_| None)),
            transaction_idx: 0,
            transactions: Vec::new(),
        }
    }

    pub fn read(&self, addr: usize, size: usize) -> Vec<u8> {
        assert!(size > 0);
        self.data[addr..addr + size].to_vec()
    }

    fn read_transaction_ids(&self, addr: usize, size: usize) -> Vec<Option<NonZeroU32>> {
        assert!(size > 0);
        self.transaction_ids[addr..addr + size].to_vec()
    }

    fn write_data(&mut self, addr: usize, data: &[u8]) {
        for (mem_cell, value) in self.data[addr..].iter_mut().zip(data.iter()) {
            *mem_cell = *value;
        }
    }

    fn write_transaction_ids(&mut self, addr: usize, transaction_ids: &[Option<NonZeroU32>]) {
        for (id_cell, value) in self.transaction_ids[addr..]
            .iter_mut()
            .zip(transaction_ids.iter())
        {
            *id_cell = *value;
        }
    }

    fn next_transaction(&mut self) -> Option<()> {
        let Some(original_transaction) = self.transactions.get_mut(self.transaction_idx) else {
            return None;
        };
        let transaction_idx = NonZeroU32::new((self.transaction_idx + 1) as u32);
        let transaction = std::mem::take(original_transaction);
        self.write_data(transaction.addr, &transaction.data);
        self.write_transaction_ids(
            transaction.addr,
            &vec![transaction_idx; transaction.data.len()],
        );
        let _ = std::mem::replace(&mut self.transactions[self.transaction_idx], transaction);
        self.transaction_idx = self.transaction_idx + 1;
        Some(())
    }

    fn previous_transaction(&mut self) -> Option<()> {
        if self.transaction_idx == 0 {
            return None;
        }
        let Some(original_transaction) = self.transactions.get_mut(self.transaction_idx - 1) else {
            return None;
        };
        let transaction = std::mem::take(original_transaction);
        self.write_data(transaction.addr, &transaction.old_data);
        self.write_transaction_ids(
            transaction.addr,
            &transaction.old_ids,
        );
        let _ = std::mem::replace(&mut self.transactions[self.transaction_idx], transaction);
        self.transaction_idx = self.transaction_idx - 1;
        Some(())
    }

    pub fn add_transaction(
        &mut self,
        addr: usize,
        data: Vec<u8>,
        code_location: usize,
    ) -> Result<(), ()> {
        if self.transactions.len() != self.transaction_idx {
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
        self.transactions.push(transaction);
        let result = self.next_transaction();
        debug_assert!(result.is_some());
        Ok(())
    }

    pub fn current_transaction_id(&self) -> usize {
        self.transaction_idx
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;

    fn setup_test_memory<const NUM_PAGES: usize, const PAGE_SIZE: usize>(
        default_value: u8,
    ) -> Memory<NUM_PAGES, PAGE_SIZE> {
        let continuously_filled_pages = NUM_PAGES / 2;
        let mut memory = std::array::from_fn(|_| None);

        let mut counter = 0;
        for i in 0..continuously_filled_pages {
            let mut data = [0; PAGE_SIZE];
            for idx in 0..PAGE_SIZE {
                data[idx] = counter;
                counter += 1;
            }
            let transaction_ids = [NonZeroU32::new(1); PAGE_SIZE];
            let page = Box::new(Page {
                data,
                transaction_ids,
            });
            memory[i] = Some(page);
        }
        for i in (continuously_filled_pages * 3 / 2)..NUM_PAGES {
            let mut data = [0; PAGE_SIZE];
            for idx in 0..PAGE_SIZE {
                data[idx] = counter;
                counter += 1;
            }
            let transaction_ids = [NonZeroU32::new(1); PAGE_SIZE];
            let page = Box::new(Page {
                data,
                transaction_ids,
            });
            memory[i] = Some(page);
        }
        Memory {
            default_value,
            memory,
            transactions: Vec::new(),
            transaction_idx: 0,
        }
    }

    #[test]
    fn empty_table_single_byte() {
        let memory = Memory::<8, 4>::new(0xab);
        let data = memory.read(0x2, 1);
        assert_eq!(data, vec![0xab]);
    }

    #[test]
    fn empty_table_page_border() {
        let memory = Memory::<8, 4>::new(0xab);
        let data = memory.read(0x0, 2);
        assert_eq!(data, vec![0xab, 0xab]);

        let data = memory.read(0x2, 2);
        assert_eq!(data, vec![0xab, 0xab]);
    }

    #[test]
    fn several_pages() {
        let memory = setup_test_memory::<4, 4>(0xab);
        let data = memory.read(0x0, 3);
        assert_eq!(data, vec![0, 1, 2]);

        let data = memory.read(0x1, 3);
        assert_eq!(data, vec![1, 2, 3]);

        let data = memory.read(0x2, 3);
        assert_eq!(data, vec![2, 3, 4]);

        let data = memory.read(0x2, 5);
        assert_eq!(data, vec![2, 3, 4, 5, 6]);

        let data = memory.read(0x4, 5);
        assert_eq!(data, vec![4, 5, 6, 7, 0xab]);

        let data = memory.read(0x7, 6);
        assert_eq!(data, vec![7, 0xab, 0xab, 0xab, 0xab, 8]);
    }

    #[test]
    fn write_in_page() {
        let mut memory = Memory::<4, 4>::new(0xab);
        let data = memory.read(0x0, 3);
        assert_eq!(data, vec![0xab, 0xab, 0xab]);
        memory.write_data(0x0, &vec![0, 1, 2]);

        let data = memory.read(0x0, 3);
        assert_eq!(data, vec![0, 1, 2]);
        let data = memory.read(0x0, 4);
        assert_eq!(data, vec![0, 1, 2, 0xab]);

        memory.write_data(0x1, &vec![0, 1, 2]);

        let data = memory.read(0x0, 4);
        assert_eq!(data, vec![0, 0, 1, 2]);
    }

    #[test]
    fn write_several_pages() {
        let mut memory = Memory::<4, 4>::new(0xab);
        let data = memory.read(0x0, 3);
        assert_eq!(data, vec![0xab, 0xab, 0xab]);
        memory.write_data(0x2, &vec![0, 1, 2]);

        let data = memory.read(0x0, 8);
        assert_eq!(data, vec![0xab, 0xab, 0, 1, 2, 0xab, 0xab, 0xab]);

        memory.write_data(0x3, &vec![0, 1, 2, 3, 4, 5, 6, 7]);

        let data = memory.read(0x0, 12);
        assert_eq!(data, vec![0xab, 0xab, 0, 0, 1, 2, 3, 4, 5, 6, 7, 0xab]);
    }

    #[test]
    fn write_ids_in_page() {
        let mut memory = Memory::<4, 4>::new(0xab);
        let transaction_ids = memory.read_transaction_ids(0x0, 3);
        assert_eq!(transaction_ids, vec![None, None, None]);

        let expected_ids = vec![NonZeroU32::new(0), NonZeroU32::new(1), NonZeroU32::new(2)];
        memory.write_data(0x0, &vec![0, 1, 2]);
        memory.write_transaction_ids(0x0, &expected_ids);
        let transaction_ids = memory.read_transaction_ids(0x0, 3);
        assert_eq!(transaction_ids, expected_ids);

        let expected_ids = vec![
            NonZeroU32::new(0),
            NonZeroU32::new(1),
            NonZeroU32::new(2),
            NonZeroU32::new(0),
        ];
        let transaction_ids = memory.read_transaction_ids(0x0, 4);
        assert_eq!(transaction_ids, expected_ids);

        let expected_ids = vec![NonZeroU32::new(0), NonZeroU32::new(1), NonZeroU32::new(2)];
        memory.write_transaction_ids(0x1, &expected_ids);

        let expected_ids = vec![
            NonZeroU32::new(0),
            NonZeroU32::new(0),
            NonZeroU32::new(1),
            NonZeroU32::new(2),
        ];
        let transaction_ids = memory.read_transaction_ids(0x0, 4);
        assert_eq!(transaction_ids, expected_ids);
    }

    #[test]
    fn write_ids_several_pages() {
        let mut memory = Memory::<4, 4>::new(0xab);
        memory.write_data(0x2, &vec![0, 1, 2]);
        memory.write_transaction_ids(
            0x2,
            &vec![NonZeroU32::new(1); 3],
        );

        let data = memory.read_transaction_ids(0x0, 8);
        assert_eq!(
            data,
            vec![
                NonZeroU32::new(0),
                NonZeroU32::new(0),
                NonZeroU32::new(1),
                NonZeroU32::new(1),
                NonZeroU32::new(1),
                NonZeroU32::new(0),
                NonZeroU32::new(0),
                NonZeroU32::new(0)
            ]
        );

        memory.write_data(0x3, &vec![0, 1, 2, 3, 4, 5, 6, 7]);
        memory.write_transaction_ids(0x3, &vec![NonZeroU32::new(2); 8]);

        let data = memory.read_transaction_ids(0x0, 12);
        assert_eq!(
            data,
            vec![
                NonZeroU32::new(0),
                NonZeroU32::new(0),
                NonZeroU32::new(1),
                NonZeroU32::new(2),
                NonZeroU32::new(2),
                NonZeroU32::new(2),
                NonZeroU32::new(2),
                NonZeroU32::new(2),
                NonZeroU32::new(2),
                NonZeroU32::new(2),
                NonZeroU32::new(2),
                NonZeroU32::new(0)
            ]
        );
    }

    #[test]
    fn apply_transaction() {
        let mut memory = Memory::<4, 4>::new(0xab);
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
            NonZeroU32::new(0),
            NonZeroU32::new(1),
            NonZeroU32::new(1),
            NonZeroU32::new(2),
            NonZeroU32::new(2),
            NonZeroU32::new(2),
            NonZeroU32::new(2),
            NonZeroU32::new(0),
        ];
        assert_eq!(result_tr, expected_result_tr);
    }

    #[test]
    fn revert_transaction() {
        let mut memory = Memory::<4, 4>::new(0xab);
        let data1 = vec![0, 1, 2, 3, 4];
        memory.add_transaction(0x1, data1.clone(), 0x0).unwrap();
        assert_eq!(memory.read(0x1, data1.len()), data1);

        let data2 = vec![4, 3, 2, 1];
        memory.add_transaction(0x3, data2.clone(), 0x0).unwrap();

        assert!(memory.previous_transaction().is_some());

        let result = memory.read(0x0, 8);
        let expected_result = vec![0xab, 0, 1, 2, 3, 4, 0xab, 0xab];
        assert_eq!(result, expected_result);
        let result_tr = memory.read_transaction_ids(0x0, 8);
        assert_eq!(result_tr.len(), 8);
        let expected_result_tr = vec![
            NonZeroU32::new(0),
            NonZeroU32::new(1),
            NonZeroU32::new(1),
            NonZeroU32::new(1),
            NonZeroU32::new(1),
            NonZeroU32::new(1),
            NonZeroU32::new(0),
            NonZeroU32::new(0),
        ];
        assert_eq!(result_tr, expected_result_tr);

        assert!(memory.previous_transaction().is_some());

        let result = memory.read(0x0, 8);
        let expected_result = vec![0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab, 0xab];
        assert_eq!(result, expected_result);
        let result_tr = memory.read_transaction_ids(0x0, 8);
        assert_eq!(result_tr.len(), 8);
        let expected_result_tr = vec![
            NonZeroU32::new(0),
            NonZeroU32::new(0),
            NonZeroU32::new(0),
            NonZeroU32::new(0),
            NonZeroU32::new(0),
            NonZeroU32::new(0),
            NonZeroU32::new(0),
            NonZeroU32::new(0),
        ];
        assert_eq!(result_tr, expected_result_tr);
    }
}
