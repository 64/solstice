use crate::{ds::SpinLock, mm::pmm::PhysAllocator};
use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use x86_64::VirtAddr;

// TODO: Use iterators. Could do with a general cleanup
// TODO: Use Layout functions to support arbitrary alignments

pub struct SlobAllocator(SpinLock<Option<NonNull<Block>>>);

unsafe impl Send for SlobAllocator {}
unsafe impl Sync for SlobAllocator {}

#[allow(unused)]
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
                debug!("Block at {:p}: size = {}, next = {:?}", curr, size, next);
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
        core::mem::align_of::<Block>(),
        core::mem::size_of::<Block>(),
    )
    .and_then(|l| l.pad_to_align())
    .and_then(|l| l.extend(layout))
    .expect("block layout creation failed");
    debug_assert_eq!(offset, core::mem::size_of::<Block>());
    let alloc_size = layout.pad_to_align().unwrap().size();

    for _ in 0..2 {
        let mut prev: Option<NonNull<Block>> = None;
        let mut curr_opt = *head;
        while let Some(mut curr) = curr_opt {
            if curr.as_mut().size == alloc_size {
                match prev {
                    Some(mut p) => p.as_mut().next = curr.as_mut().next,
                    None => *head = curr.as_mut().next,
                }

                return Block::allocation(curr);
            } else if curr.as_mut().size > alloc_size {
                let (left, right) = Block::split_at(curr, alloc_size);
                if let Some(right) = right {
                    match prev {
                        Some(mut p) => p.as_mut().next = Some(right),
                        None => *head = Some(right),
                    }
                }

                return Block::allocation(left);
            }

            prev = curr_opt;
            curr_opt = curr.as_mut().next;
        }

        morecore(head);
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

fn morecore(head: &mut Option<NonNull<Block>>) {
    unsafe {
        let addr: VirtAddr = PhysAllocator::alloc(0).start.start_address().into();
        let p_block = addr.as_mut_ptr::<Block>();
        let size = super::PAGE_SIZE - core::mem::size_of::<Block>();
        (*p_block).size = size;
        (*p_block).next = None;

        // Put this new chunk onto the free list
        dealloc_inner(
            head,
            p_block.offset(1) as *mut u8,
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
        bytes: usize,
    ) -> (NonNull<Self>, Option<NonNull<Self>>) {
        debug_assert!(block.as_mut().size >= bytes);

        let remaining = block.as_mut().size - bytes;
        let next = if remaining > core::mem::size_of::<Block>() {
            let mut next = Block::offset_addr(block, bytes);
            next.as_mut().size = remaining;
            next.as_mut().next = block.as_mut().next;
            block.as_mut().size = bytes;

            Some(next)
        } else {
            None
        };

        block.as_mut().next = next;
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
