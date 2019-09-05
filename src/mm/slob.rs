use crate::{ds::SpinLock, mm::pmm::PhysAllocator};
use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use x86_64::VirtAddr;

// TODO: Use iterators. Could do with a general cleanup

pub struct SlobAllocator(SpinLock<Option<NonNull<Block>>>);

unsafe impl Send for SlobAllocator {}
unsafe impl Sync for SlobAllocator {}

#[repr(align(16))]
struct Block {
    size: usize,
    next: Option<NonNull<Block>>,
}

unsafe impl Send for Block {}

#[global_allocator]
static HEAP: SlobAllocator = SlobAllocator::new();

impl SlobAllocator {
    const fn new() -> Self {
        Self(SpinLock::new(None))
    }

    #[allow(unused)]
    pub fn debug() {
        let list = HEAP.0.lock();

        if list.is_none() {
            debug!("HEAP: None");
        }

        let mut curr_opt = *list;
        while let Some(mut curr) = curr_opt {
            unsafe {
                let size = curr.as_mut().size;
                let next = curr.as_mut().next;
                debug_assert_ne!(next, Some(curr), "cycle in free list");
                debug!(
                    "Block at {:p}: size = {:#x}, next = {:?}",
                    Block::allocation(curr),
                    size,
                    next
                );
                curr_opt = curr.as_mut().next;
            }
        }
    }
}

unsafe impl GlobalAlloc for SlobAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        alloc_inner(&mut *self.0.lock(), layout)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        dealloc_inner(&mut *self.0.lock(), ptr, layout);
    }
}

unsafe fn alloc_inner(head: &mut Option<NonNull<Block>>, layout: Layout) -> *mut u8 {
    let (layout, offset) = Layout::from_size_align(
        core::mem::size_of::<Block>(),
        core::mem::align_of::<Block>(),
    )
    .and_then(|l| l.pad_to_align())
    .and_then(|l| l.extend(layout))
    .and_then(|(l, o)| l.pad_to_align().map(|l| (l, o)))
    .expect("block layout creation failed");

    let alloc_len = layout.size() - offset;

    debug_assert_eq!(offset, core::mem::size_of::<Block>());

    if layout.size() > super::PAGE_SIZE {
        trace!("slob: large {} byte allocation", layout.size());
    }

    for _ in 0..2 {
        let mut prev: Option<NonNull<Block>> = None;
        let mut curr_opt = *head;
        while let Some(mut curr) = curr_opt {
            if curr.as_mut().size == alloc_len {
                match prev {
                    Some(mut p) => p.as_mut().next = curr.as_mut().next,
                    None => *head = curr.as_mut().next,
                }

                return Block::allocation(curr);
            } else if curr.as_mut().size > layout.size() {
                let (left, right) = Block::split_at(curr, alloc_len);
                match prev {
                    Some(mut p) => p.as_mut().next = Some(right),
                    None => *head = Some(right),
                }

                return Block::allocation(left);
            }

            prev = curr_opt;
            curr_opt = curr.as_mut().next;
        }

        morecore(head, (layout.size() + super::PAGE_SIZE) / super::PAGE_SIZE);
    }

    unreachable!();
}

unsafe fn dealloc_inner(head: &mut Option<NonNull<Block>>, ptr: *mut u8, layout: Layout) {
    let mut block = Block::from_allocation(ptr);
    let mut prev: Option<NonNull<Block>> = None;
    let mut curr_opt = *head;
    while let Some(mut curr) = curr_opt {
        assert_ne!(curr, block, "double free of ptr {:p}, {:?}", ptr, layout);

        if curr > block {
            // Insert between prev and curr
            block.as_mut().next = Some(curr);
            match prev {
                Some(mut p) => {
                    // TODO: Free physical pages when possible
                    p.as_mut().next = Some(block);
                    if Block::try_merge(p, block) {
                        block = p;
                    }
                }
                None => *head = Some(block),
            }

            Block::try_merge(block, curr);
            return;
        }

        prev = curr_opt;
        curr_opt = curr.as_mut().next;
    }

    // Insert at the end
    match prev {
        Some(mut p) => {
            p.as_mut().next = Some(block);
            block.as_mut().next = None;
            Block::try_merge(p, block);
        }
        None => *head = Some(block),
    }
}

fn morecore(head: &mut Option<NonNull<Block>>, num_pages: usize) {
    unsafe {
        let addr: VirtAddr =
            PhysAllocator::alloc(num_pages.next_power_of_two().trailing_zeros() as u8)
                .start
                .start_address()
                .into();
        let p_block = addr.as_mut_ptr::<Block>();
        let size = num_pages * super::PAGE_SIZE - core::mem::size_of::<Block>();
        (*p_block).size = size;
        (*p_block).next = None;

        // Put this new chunk onto the free list
        dealloc_inner(
            head,
            Block::allocation(NonNull::new(p_block).unwrap()),
            Layout::from_size_align(size, 1).unwrap(),
        );
    }
}

impl Block {
    unsafe fn offset_addr(block: NonNull<Block>, size: usize) -> NonNull<Block> {
        let out =
            (block.as_ptr() as *mut u8).offset((size + core::mem::size_of::<Block>()) as isize);
        NonNull::new(out as *mut Block).unwrap()
    }

    unsafe fn split_at(
        mut block: NonNull<Self>,
        alloc_len: usize,
    ) -> (NonNull<Self>, NonNull<Self>) {
        let total_len = alloc_len + core::mem::size_of::<Block>();
        debug_assert!(block.as_mut().size >= total_len);

        let remaining = block.as_mut().size - total_len;
        let mut next = Block::offset_addr(block, alloc_len);

        next.as_mut().size = remaining;
        next.as_mut().next = block.as_mut().next;
        block.as_mut().size = alloc_len;
        block.as_mut().next = Some(next);
        (block, next)
    }

    unsafe fn try_merge(mut left: NonNull<Self>, mut right: NonNull<Self>) -> bool {
        debug_assert!(left < right);
        let mask = !(super::PAGE_SIZE - 1);
        if left.as_ptr() as usize & mask != right.as_ptr() as usize & mask {
            // The blocks are in separate pages. Since we allocate each physical page as
            // order 0, we can't merge these.
            false
        } else if Block::offset_addr(left, left.as_mut().size) == right {
            // Merge
            left.as_mut().size += right.as_mut().size + core::mem::size_of::<Block>();
            left.as_mut().next = right.as_mut().next;
            // TODO: Occasionally walk the list and free a physical page?
            true
        } else {
            false
        }
    }

    fn allocation(block: NonNull<Self>) -> *mut u8 {
        unsafe { block.as_ptr().offset(1) as *mut u8 }
    }

    fn from_allocation(ptr: *mut u8) -> NonNull<Self> {
        unsafe { NonNull::new((ptr as *mut Block).offset(-1)).unwrap() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    test_case!(basic_alloc, {
        use alloc::boxed::Box;
        let mut x = Box::new(0);
        *x += 2;
    });

    test_case!(repeated_allocs, {
        use alloc::boxed::Box;
        for _ in 0..20 {
            let mut x = Box::new(0);
            *x += 2;
        }
    });
}
