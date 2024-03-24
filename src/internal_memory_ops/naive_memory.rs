use std::num::NonZeroU32;

use super::InternalMemoryOps;
use super::Transaction;
use crate::Memory;

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct NaiveMemory<const NUM_PAGES: usize, const PAGE_SIZE: usize, const SIZE: usize> {
    default_value: u8,
    data: Box<[u8; SIZE]>,
    transaction_ids: Box<[Option<NonZeroU32>; SIZE]>,
    transactions: Vec<Transaction>,
    transaction_idx: usize,
}

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
}

impl<const NUM_PAGES: usize, const PAGE_SIZE: usize, const SIZE: usize> InternalMemoryOps
    for NaiveMemory<NUM_PAGES, PAGE_SIZE, SIZE>
{
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
    fn transaction_vec_len(&self) -> usize {
        self.transactions.len()
    }
    fn transaction_vec_push(&mut self, transaction: Transaction) {
        self.transactions.push(transaction)
    }
    fn get_mut_transaction(&mut self, idx: usize) -> Option<&mut Transaction> {
        self.transactions.get_mut(idx)
    }
    fn set_transaction_idx(&mut self, idx: usize) {
        self.transaction_idx = idx;
    }
}

impl<const NUM_PAGES: usize, const PAGE_SIZE: usize, const SIZE: usize> Memory
    for NaiveMemory<NUM_PAGES, PAGE_SIZE, SIZE>
{
    fn read(&self, addr: usize, size: usize) -> Vec<u8> {
        assert!(size > 0);
        self.data[addr..addr + size].to_vec()
    }

    fn read_transaction_ids(&self, addr: usize, size: usize) -> Vec<Option<NonZeroU32>> {
        assert!(size > 0);
        self.transaction_ids[addr..addr + size].to_vec()
    }

    fn current_transaction_id(&self) -> usize {
        self.transaction_idx
    }
}
