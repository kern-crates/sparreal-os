use core::{alloc::Layout, arch::asm, cell::UnsafeCell, ptr::NonNull, sync::atomic::AtomicU64};

use aarch64_cpu::{asm::barrier, registers::*};
use page_table::{
    aarch64::{flush_tlb, DescriptorAttr, PTE},
    Access, PhysAddr, VirtAddr,
};
use tock_registers::interfaces::ReadWriteable;

use crate::KernelConfig;

const BYTES_1G: usize = 1024 * 1024 * 1024;

const MAIR_VALUE: u64 = {
    // Device-nGnRE memory
    let attr0 = MAIR_EL1::Attr0_Device::nonGathering_nonReordering_EarlyWriteAck.value;
    // Normal memory
    let attr1 = MAIR_EL1::Attr1_Normal_Inner::WriteBack_NonTransient_ReadWriteAlloc.value
        | MAIR_EL1::Attr1_Normal_Outer::WriteBack_NonTransient_ReadWriteAlloc.value;
    let attr2 = MAIR_EL1::Attr2_Normal_Inner::NonCacheable.value
        | MAIR_EL1::Attr2_Normal_Outer::NonCacheable.value;
    attr0 | attr1 | attr2 // 0x44_ff_04
};

pub type PageTableRef = page_table::PageTableRef<PTE, 4>;

#[allow(unused)]
#[repr(C)]
enum AttrIndex {
    Device = 0,
    Normal = 1,
    NonCacheable = 2,
}

pub struct MMU {
    va_offset: UnsafeCell<usize>,
}

unsafe impl Send for MMU {}
unsafe impl Sync for MMU {}

impl MMU {
    pub const fn new() -> MMU {
        MMU {
            va_offset: UnsafeCell::new(0),
        }
    }

    pub unsafe fn enable(&self, cfg: &KernelConfig) {
        let va_offset = cfg.va_offset;
        self.va_offset.get().write(va_offset);

        let mut access = BeforeMMUPageAllocator::new(cfg.heap_lma.as_ptr() as usize, 1024 * 4096);

        let mut table = PageTableRef::try_new(&mut access).unwrap();

        let virt_p = VirtAddr::from(cfg.kernel_lma.as_ptr() as usize).align_down(BYTES_1G);
        let phys = PhysAddr::from(virt_p.as_usize());
        let virt = virt_p + va_offset;

        let _ = table.map_region(
            virt_p,
            phys,
            BYTES_1G,
            DescriptorAttr::new(AttrIndex::Normal as u64) | DescriptorAttr::UXN,
            true,
            &mut access,
        );
        let _ = table.map_region(
            virt,
            phys,
            BYTES_1G,
            DescriptorAttr::new(AttrIndex::Normal as u64) | DescriptorAttr::UXN,
            true,
            &mut access,
        );
        let root_paddr = table.paddr().as_usize() as _;

        MAIR_EL1.set(MAIR_VALUE);

        // Enable TTBR0 and TTBR1 walks, page size = 4K, vaddr size = 48 bits, paddr size = 40 bits.
        let tcr_flags0 = TCR_EL1::EPD0::EnableTTBR0Walks
            + TCR_EL1::TG0::KiB_4
            + TCR_EL1::SH0::Inner
            + TCR_EL1::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::T0SZ.val(16);
        let tcr_flags1 = TCR_EL1::EPD1::EnableTTBR1Walks
            + TCR_EL1::TG1::KiB_4
            + TCR_EL1::SH1::Inner
            + TCR_EL1::ORGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::IRGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable
            + TCR_EL1::T1SZ.val(16);
        TCR_EL1.write(TCR_EL1::IPS::Bits_48 + tcr_flags0 + tcr_flags1);
        barrier::isb(barrier::SY);

        // Set both TTBR0 and TTBR1
        TTBR1_EL1.set_baddr(root_paddr);
        TTBR0_EL1.set_baddr(root_paddr);

        // Enable the MMU and turn on I-cache and D-cache
        SCTLR_EL1.modify(SCTLR_EL1::M::Enable + SCTLR_EL1::C::Cacheable + SCTLR_EL1::I::Cacheable);
        asm!("
    ADD  sp, sp, {0}
    ADD  x30, x30, {0}
    ", in(reg) va_offset);

        flush_tlb(None);
        barrier::isb(barrier::SY);


    }

    pub fn va_offset(&self) -> usize {
        unsafe { *self.va_offset.get() }
    }
}

struct BeforeMMUPageAllocator {
    end: usize,
    iter: usize,
}

impl BeforeMMUPageAllocator {
    unsafe fn new(start: usize, size: usize) -> Self {
        Self {
            iter: start,
            end: start + size,
        }
    }
}

impl Access for BeforeMMUPageAllocator {
    unsafe fn alloc(&mut self, layout: Layout) -> Option<NonNull<u8>> {
        let size = layout.size();
        if self.iter + size > self.end {
            return None;
        }
        let ptr = self.iter;
        self.iter += size;
        NonNull::new(ptr as *mut u8)
    }

    unsafe fn dealloc(&mut self, ptr: NonNull<u8>, layout: Layout) {}

    fn virt_to_phys<T>(&self, addr: NonNull<T>) -> usize {
        addr.as_ptr() as usize
    }

    fn phys_to_virt<T>(&self, phys: usize) -> NonNull<T> {
        unsafe { NonNull::new_unchecked(phys as *mut T) }
    }
}
