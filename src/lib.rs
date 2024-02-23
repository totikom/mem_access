use std::num::NonZeroU32;

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

struct Transaction {
    address: usize,
    data: Vec<u8>,
    old_data: Vec<u8>,
    old_ids: Vec<Option<NonZeroU32>>,
}

pub struct Memory<const NUM_PAGES: usize, const PAGE_SIZE: usize> {
    default_value: u8,
    memory: [Option<Box<Page<PAGE_SIZE>>>; NUM_PAGES],
    transactions: Vec<Transaction>,
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
        }
    }

    fn get_page_data(
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

        let mut data = Vec::new();
        if start_page_addr == end_page_addr {
            data = self.get_page_data(start_page_addr, in_page_start_addr, in_page_end_addr);
        } else {
            data = self.get_page_data(start_page_addr, in_page_start_addr, PAGE_SIZE-1);
            for page_idx in start_page_addr + 1..end_page_addr {
                data.extend(self.get_page_data(page_idx, 0, PAGE_SIZE-1));
            }
            data.extend(self.get_page_data(end_page_addr, 0, in_page_end_addr));
        }
        data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let data = memory.read(0x2, 5);
        assert_eq!(data, vec![2, 3, 4, 5, 6]);

        let data = memory.read(0x4, 5);
        assert_eq!(data, vec![4, 5, 6, 7, 0xab]);

        let data = memory.read(0x7, 6);
        assert_eq!(data, vec![7, 0xab, 0xab, 0xab, 0xab, 8]);
    }
}
