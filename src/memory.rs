use bootloader::bootinfo::{MemoryMap, MemoryRegionType};
use x86_64::{
    registers::control::Cr3,
    structures::paging::{
        page_table::FrameError, FrameAllocator, Mapper, OffsetPageTable, Page, PageTable,
        PhysFrame, Size4KiB,
    },
    PhysAddr, VirtAddr,
};

/// 一个从bootloader内存映射中返回可用帧的帧分配器
pub struct BootInfoFrameAllocator {
    memory_map: &'static MemoryMap,
    next: usize,
}

impl BootInfoFrameAllocator {
    /// 从传入的内存映射中创建帧分配器
    ///
    /// 该函数为非安全，因为调用者必须确保传入的内存映射是有效的。
    /// 主要要求是其中所有标记为`USABLE`的帧实际上都未被使用。
    pub unsafe fn init(memory_map: &'static MemoryMap) -> Self {
        BootInfoFrameAllocator {
            memory_map,
            next: 0,
        }
    }

    /// 返回内存映射中可用帧的迭代器
    fn usable_frames(&self) -> impl Iterator<Item = PhysFrame> {
        // 获取内存映射中的可用区域
        let regions = self.memory_map.iter();
        let usable_regions = regions.filter(|r| r.region_type == MemoryRegionType::Usable);
        // 将各区域化为其地址范围
        let addr_ranges = usable_regions.map(|r| r.range.start_addr()..r.range.end_addr());
        // 将这些帧的起始地址化为迭代器
        let frame_addresses = addr_ranges.flat_map(|r| r.step_by(4096));
        // 使用这些起始地址创建`PhysFrame`类型
        frame_addresses.map(|addr| PhysFrame::containing_address(PhysAddr::new(addr)))
    }
}

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        let frame = self.usable_frames().nth(self.next);
        self.next += 1;
        frame
    }
}

// 初始化OffsetPageTable
///
/// 该函数为非安全，因为调用者必须保证已将完整的物理内存
/// 映射到偏移量为`physical_memory_offset`的虚拟内存中了。
/// 同时，该函数只能被调用一次，以避免产生其他`＆mut`引用（可能会造成未定义的行为）
pub unsafe fn init(physical_memory_offset: VirtAddr) -> OffsetPageTable<'static> {
    let level_4_table = active_level_4_table(physical_memory_offset);
    OffsetPageTable::new(level_4_table, physical_memory_offset)
}

/// 返回4级页表的可变引用
///
/// 该函数为非安全，因为调用者必须保证已将完整的物理内存
/// 映射到偏移量为`physical_memory_offset`的虚拟内存中了。
/// 同时，该函数只能被调用一次，以避免产生其他`＆mut`引用（可能会造成未定义的行为）。
unsafe fn active_level_4_table(physical_memory_offset: VirtAddr) -> &'static mut PageTable {
    use x86_64::registers::control::Cr3;

    let (level_4_table_frame, _) = Cr3::read();

    let phys = level_4_table_frame.start_address();
    let virt = physical_memory_offset + phys.as_u64();
    let page_table_ptr: *mut PageTable = virt.as_mut_ptr();

    &mut *page_table_ptr
}

pub unsafe fn translate_addr(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    translate_addr_inner(addr, physical_memory_offset)
}

/// `translate_addr`调用的私有函数
///
/// 该函数可以安全地限制`unsafe`操作的范围，因为Rust将非安全函数整体视为非安全块。
/// 该函数只能通过该模块外部的`unsafe fn`来访问。
fn translate_addr_inner(addr: VirtAddr, physical_memory_offset: VirtAddr) -> Option<PhysAddr> {
    let (level_4_table_frame, _) = Cr3::read();

    let table_indexes = [addr.p4_index(), addr.p2_index(), addr.p1_index()];

    let mut frame = level_4_table_frame;

    for &index in &table_indexes {
        let virt = physical_memory_offset + frame.start_address().as_u64();
        let table_ptr: *const PageTable = virt.as_ptr();
        let table = unsafe { &*table_ptr };

        let entry = &table[index];
        frame = match entry.frame() {
            Ok(frame) => frame,
            Err(FrameError::FrameNotPresent) => return None,
            Err(FrameError::HugeFrame) => panic!("huge pages not supported"),
        };
    }
    Some(frame.start_address() + u64::from(addr.page_offset()))
}

/// 一个始终返回`None`的帧分配器
pub struct EmptyFrameAllocator;

unsafe impl FrameAllocator<Size4KiB> for EmptyFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        None
    }
}

pub fn create_example_mapping(
    page: Page,
    mapper: &mut OffsetPageTable,
    frame_allocator: &mut impl FrameAllocator<Size4KiB>,
) {
    use x86_64::structures::paging::PageTableFlags as Flags;

    let frame = PhysFrame::containing_address(PhysAddr::new(0xb8000));
    let flags = Flags::PRESENT | Flags::WRITABLE;

    let map_to_result = unsafe {
        // FIXME: 不安全用法，仅演示用
        mapper.map_to(page, frame, flags, frame_allocator)
    };
    map_to_result.expect("map_to failed").flush();
}
