use crate::{ds::SpinLock, mm::pmm::PhysAllocator};
use alloc::alloc::{GlobalAlloc, Layout};
use core::ptr::NonNull;
use x86_64::VirtAddr;

// TODO: Use iterators. Could do with a general cleanup

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
        dbg!(alloc_inner(&mut *self.0.lock(), layout))
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        dealloc_inner(&mut *self.0.lock(), ptr, layout);
    }
}

unsafe fn alloc_inner(head: &mut Option<NonNull<Block>>, layout: Layout) -> *mut u8 {
    debug_assert!(layout.align() <= core::mem::align_of::<Block>());
    let alloc_size = layout
        .align_to(core::mem::align_of::<Block>())
        .unwrap()
        .pad_to_align()
        .unwrap()
        .size();

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
            match prev {
                Some(mut p) => {
                    // TODO: Don't merge across page boundaries
                    // TODO: Free physical pages when possible
                    if Block::offset_addr(p, p.as_mut().size) == block {
                        // Merge prev and block
                        p.as_mut().size += core::mem::size_of::<Block>() + block.as_mut().size;
                        block = p;
                    } else {
                        p.as_mut().next = Some(block);
                    }
                }
                None => *head = Some(block),
            }

            if Block::offset_addr(block, block.as_mut().size) == curr {
                // Merge block and curr
                // Prev already points to block here
                block.as_mut().size += core::mem::size_of::<Block>() + curr.as_mut().size;
            }

            block.as_mut().next = Some(curr)
        }

        prev = curr_opt;
        curr_opt = curr.as_mut().next;
    }

    // Insert at the end
    match prev {
        Some(mut p) => {
            // TODO: Don't merge across page boundaries, free physical pages
            if Block::offset_addr(p, p.as_mut().size) == block {
                // Merge prev and block
                p.as_mut().size += core::mem::size_of::<Block>() + block.as_mut().size;
            } else {
                p.as_mut().next = Some(block);
                block.as_mut().next = None;
            }
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
        {
            let mut x = Box::new(0);
            *x += 2;
        }

        SlobAllocator::debug();
    });

    test_case!(repeated_allocs, {
        use alloc::boxed::Box;
        for _ in 0..20 {
            let mut x = Box::new(0);
            *x += 2;
        }
    });
}
