use core::ptr::NonNull;

use page_table_generic::{err::PagingResult, Access, AccessSetting, CacheSetting, MapConfig};

use crate::{dbg, dbg_hex, dbg_hexln, dbgln, MemoryRange};

use super::{table::PageTableRef, MemoryReservedRange, PageAllocator};

#[derive(Clone)]
pub struct BootTableConfig {
    /// Kernel 所在的内存
    pub main_memory: MemoryRange,
    /// 已使用的内存
    pub main_memory_heap_offset: usize,
    /// 每核 Kernel sp 大小
    pub hart_stack_size: usize,
    /// 需要提前map的内存
    pub reserved_memory: [Option<MemoryReservedRange>; 24],
    /// 物理内存和虚拟内存的偏移
    pub va_offset: usize,
}

pub fn new_boot_table(config: BootTableConfig) -> PagingResult<usize> {
    let heap_size =
        (config.main_memory.size - config.main_memory_heap_offset - config.hart_stack_size) / 2;
    let heap_start = config.main_memory.start + config.main_memory_heap_offset + heap_size;

    let mut access = unsafe {
        PageAllocator::new(
            NonNull::new_unchecked(heap_start.as_usize() as _),
            heap_size,
        )
    };

    let mut table = PageTableRef::create_empty(&mut access)?;
    dbg_hexln!(config.va_offset);

    unsafe {
        map_boot_region(
            "main memory",
            &mut table,
            config.main_memory.start.into(),
            config.main_memory.size,
            AccessSetting::Read | AccessSetting::Write | AccessSetting::Execute,
            CacheSetting::Normal,
            &mut access,
            config.va_offset,
        )?;

        for rsv in config.reserved_memory {
            if let Some(rsv) = rsv {
                map_boot_region(
                    rsv.name,
                    &mut table,
                    rsv.start.into(),
                    rsv.size,
                    rsv.access,
                    rsv.cache,
                    &mut access,
                    config.va_offset,
                )?;
            }
        }
    }
    Ok(table.paddr())
}

unsafe fn map_boot_region(
    name: &str,
    table: &mut PageTableRef<'_>,
    paddr: usize,
    size: usize,
    access_setting: AccessSetting,
    cache_setting: CacheSetting,
    access: &mut impl Access,
    va_offset: usize,
) -> PagingResult<()> {
    map_boot_region_once(
        name,
        table,
        paddr,
        paddr,
        size,
        access_setting,
        cache_setting,
        access,
    )?;

    map_boot_region_once(
        name,
        table,
        paddr + va_offset,
        paddr,
        size,
        access_setting,
        cache_setting,
        access,
    )?;

    Ok(())
}

unsafe fn map_boot_region_once(
    name: &str,
    table: &mut PageTableRef<'_>,
    vaddr: usize,
    paddr: usize,
    size: usize,
    access_setting: AccessSetting,
    cache_setting: CacheSetting,
    access: &mut impl Access,
) -> PagingResult<()> {
    let name_count = name.chars().count();
    let name_space = 12;
    let space = if name_space > name_count {
        name_space - name_count
    } else {
        0
    };
    dbg!("Map [");
    dbg!(name);
    for _ in 0..space {
        dbg!(" ");
    }
    dbg!("]: [");
    dbg_hex!(vaddr);
    dbg!(", ");
    dbg_hex!(vaddr + size);
    dbg!(") -> [");
    dbg_hex!(paddr);
    dbg!(", ");
    dbg_hex!(paddr + size);
    dbgln!(")");

    table.map_region(
        MapConfig::new(vaddr as _, paddr, access_setting, cache_setting),
        size,
        true,
        access,
    )
}
