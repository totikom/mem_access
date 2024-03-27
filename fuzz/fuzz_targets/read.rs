#![no_main]

use arbitrary::Arbitrary;
use libfuzzer_sys::fuzz_target;
use pretty_assertions::assert_eq;

use mem_access::{Memory, NaiveMemory, PagedMemory};

#[derive(Arbitrary, Debug)]
struct MemoryWrite {
    addr: usize,
    data: Vec<u8>,
}
#[derive(Arbitrary, Debug)]
struct MemoryRead {
    addr: usize,
    size: usize,
}

#[derive(Arbitrary, Debug)]
struct FuzzData {
    writes: Vec<MemoryWrite>,
    reads: Vec<MemoryRead>,
}

fuzz_target!(|fuzz_data: FuzzData| {
    let mut paged_memory = PagedMemory::<256, 256>::new(0xab);
    let mut naive_memory = NaiveMemory::<256, 256, { 256 * 256 }>::new(0xab);

    for write in fuzz_data.writes {
        let (max_addr, overflowed) = write.addr.overflowing_add(write.data.len());
        if overflowed || max_addr >= 256 * 256 {
            continue;
        } else if write.data.is_empty() {
            continue;
        }
        assert!(paged_memory
            .add_transaction(write.addr, write.data.clone(), 0)
            .is_ok());
        assert!(naive_memory
            .add_transaction(write.addr, write.data, 0)
            .is_ok());
    }
    for read in fuzz_data.reads {
        let (max_addr, overflowed) = read.addr.overflowing_add(read.size);
        if overflowed || max_addr >= 256 * 256 {
            continue;
        } else if read.size == 0 {
            continue;
        }

        assert_eq!(paged_memory.read(read.addr, read.size), naive_memory.read(read.addr, read.size));
    }
});
