use core::{
    alloc::{GlobalAlloc, Layout},
    mem, ptr,
};

use super::bump::{align_up, Locked};


struct ListNode {
    size: usize,
    next: Option<&'static mut ListNode>,
}

impl ListNode {
    const fn new(size: usize) -> Self {
        ListNode { size, next: None }
    }
    fn start_addr(&self) -> usize {
        self as *const Self as usize
    }
    fn end_addr(&self) -> usize {
        self.start_addr() + self.size
    }
}

pub struct LinkedListAllocator {
    head: ListNode,
}

impl LinkedListAllocator {
    pub const fn new() -> Self {
        Self {
            head: ListNode::new(0),
        }
    }
    /// 使用给定的堆边界初始化分配器
    ///
    /// 该方法为非安全，因为调用者必须保证提供的内存范围未被使用。
    /// 同时，该方法只能被调用一次。
    pub unsafe fn init(&mut self, heap_start: usize, heap_size: usize) {
        self.add_free_region(heap_start, heap_size);
    }
    /// 将给定的内存区域添加至链表前端
    unsafe fn add_free_region(&mut self, addr: usize, size: usize) {
        // 确保此空闲区域足以容纳一个`ListNode`
        assert_eq!(align_up(addr, mem::align_of::<ListNode>()), addr);
        assert!(size >= mem::size_of::<ListNode>());

        // 创建一个新的`ListNode`并将其添加至链表前端
        let mut node = ListNode::new(size);
        node.next = self.head.next.take();
        let node_ptr = addr as *mut ListNode;
        node_ptr.write(node);
        self.head.next = Some(&mut *node_ptr)
    }

    fn find_region(&mut self, size: usize, align: usize) -> Option<(&'static mut ListNode, usize)> {
        // 当前链表节点的引用，会在每次迭代中更新
        let mut current = &mut self.head;

        // 在链表中查找一个足够大的内存区域
        while let Some(ref mut region) = current.next {
            if let Ok(alloc_start) = Self::alloc_from_region(&region, size, align) {
                // 该区域可以容纳所需的分配，则将该区域从链表中删除
                let next = region.next.take();
                let ret = Some((current.next.take().unwrap(), alloc_start));
                current.next = next;
                return ret;
            } else {
                // 该区域不可以容纳所需的分配，则继续下一轮迭代查找
                current = current.next.as_mut().unwrap();
            }
        }

        // 已经找不到合适的内存区域了
        None
    }
    /// 尝试使用给定内存区域，为给定大小和对齐方式的分配做出分配
    ///
    /// 如果成功则返回分配的开始地址。
    fn alloc_from_region(region: &ListNode, size: usize, align: usize) -> Result<usize, ()> {
        let alloc_start = align_up(region.start_addr(), align);
        let alloc_end = alloc_start.checked_add(size).ok_or(())?;

        if alloc_end > region.end_addr() {
            // 内存区域太小
            return Err(());
        }

        let excess_size = region.end_addr() - alloc_end;
        if excess_size > 0 && excess_size < mem::size_of::<ListNode>() {
            // 该内存区域的剩余部分太小，无法容纳一个`ListNode`
            // （这是必须的，因为分配动作会将该区域分为已使用和未使用两个部分）
            return Err(());
        }

        // 该区域适合于给定的分配
        Ok(alloc_start)
    }
    /// 调整给定布局，使生成的用以分配的内存区域也能够存储`ListNode`。
    ///
    /// 以元组`(size, align)`的形式返回调整后的大小和对齐方式。
    fn size_align(layout: Layout) -> (usize, usize) {
        let layout = layout
            .align_to(mem::align_of::<ListNode>())
            .expect("adjusting alignment failed")
            .pad_to_align();
        let size = layout.size().max(mem::size_of::<ListNode>());
        (size, layout.align())
    }
}

unsafe impl GlobalAlloc for Locked<LinkedListAllocator> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let (size, align) = LinkedListAllocator::size_align(layout);
        let mut allocator = self.lock();

        if let Some((region, alloc_start)) = allocator.find_region(size, align) {
            let alloc_end = alloc_start.checked_add(size).expect("overflow");
            let excess_size = region.end_addr() - alloc_end;
            if excess_size > 0 {
                allocator.add_free_region(alloc_end, excess_size);
            }
            alloc_start as *mut u8
        } else {
            ptr::null_mut()
        }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let (size, _) = LinkedListAllocator::size_align(layout);
        self.lock().add_free_region(ptr as usize, size);
    }
}
