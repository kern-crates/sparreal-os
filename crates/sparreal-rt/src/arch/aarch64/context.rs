use core::{
    arch::asm,
    fmt::{self, Debug},
};

use sparreal_kernel::task::TaskControlBlock;

#[repr(C, align(0x10))]
#[derive(Clone)]
pub struct Context {
    pub sp: *const u8,
    pub pc: *const u8,
    #[cfg(hard_float)]
    /// Floating-point Control Register (FPCR)
    pub fpcr: usize,
    #[cfg(hard_float)]
    /// Floating-point Status Register (FPSR)
    pub fpsr: usize,
    #[cfg(hard_float)]
    pub q: [u128; 32],
    pub spsr: u64,
    pub x: [usize; 30],
    pub lr: *const u8,
}

unsafe impl Send for Context {}

impl Debug for Context {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Context:")?;

        const NUM_CHUNKS: usize = 4;

        for (r, chunk) in self.x.chunks(NUM_CHUNKS).enumerate() {
            let row_start = r * NUM_CHUNKS;

            for (i, v) in chunk.iter().enumerate() {
                let i = row_start + i;
                write!(f, "  x{:<3}: {:#18x}", i, v)?;
            }
            writeln!(f)?;
        }
        writeln!(f, "  lr  : {:p}", self.lr)?;
        writeln!(f, "  spsr: {:#18x}", self.spsr)?;
        writeln!(f, "  pc  : {:p}", self.pc)?;
        writeln!(f, "  sp  : {:p}", self.sp)
    }
}

#[inline(always)]
fn __ctx_store_x_q() {
    unsafe {
        asm!(
            "
	stp X29,X30, [sp,#-0x10]!
	stp X27,X28, [sp,#-0x10]!
    stp X25,X26, [sp,#-0x10]!
	stp X23,X24, [sp,#-0x10]!
    stp X21,X22, [sp,#-0x10]!
	stp X19,X20, [sp,#-0x10]!
	stp	X17,X18, [sp,#-0x10]!
	stp	X15,X16, [sp,#-0x10]!
	stp	X13,X14, [sp,#-0x10]!
	stp	X11,X12, [sp,#-0x10]!
	stp	X9,X10,  [sp,#-0x10]!
	stp	X7,X8,   [sp,#-0x10]!
	stp	X5,X6,   [sp,#-0x10]!
	stp	X3,X4,   [sp,#-0x10]!
    stp	X1,X2,   [sp,#-0x10]!
    mrs	x9,     SPSR_EL1
    stp x9, x0, [sp,#-0x10]!"
        );

        #[cfg(hard_float)]
        asm!(
            "
    stp q30, q31,  [sp,#-0x20]!
    stp q28, q29,  [sp,#-0x20]!
    stp q26, q27,  [sp,#-0x20]!
    stp q24, q25,  [sp,#-0x20]!
    stp q22, q23,  [sp,#-0x20]!
    stp q20, q21,  [sp,#-0x20]!
    stp q18, q19,  [sp,#-0x20]!
    stp q16, q17,  [sp,#-0x20]!
    stp q14, q15,  [sp,#-0x20]!
    stp q12, q13,  [sp,#-0x20]!
    stp q10, q11,  [sp,#-0x20]!
    stp q8,  q9,   [sp,#-0x20]!
    stp q6,  q7,   [sp,#-0x20]!
    stp q4,  q5,   [sp,#-0x20]!
    stp q2,  q3,   [sp,#-0x20]!
    stp q0,  q1,   [sp,#-0x20]!
    mrs     x9,  fpcr
    mrs     x10, fpsr
    stp x9,  x10,  [sp,#-0x10]!
        "
        );
    }
}

/// return `x9` is sp, `x10` is lr
#[inline(always)]
fn store_pc_is_lr() {
    __ctx_store_x_q();
    unsafe {
        asm!(
            "
    mov x10, lr
    mov x9, sp
    sub x9, x9,   #0x10
	stp x9, x10,  [sp,#-0x10]!
        "
        );
    }
}

#[inline(always)]
pub(super) fn store_pc_is_elr() {
    __ctx_store_x_q();
    unsafe {
        asm!(
            "
    mrs x10, ELR_EL1
    mov x9, sp
    sub x9, x9,   #0x10
	stp x9, x10,  [sp,#-0x10]!
        "
        );
    }
}

/// return `x9` is sp, `x10` is lr
#[inline(always)]
fn restore_pc_is_lr() {
    unsafe {
        asm!(
            "
    ldp x9, lr, [sp], #0x10
        "
        );
    }
    __ctx_restore_x_q();
}

#[inline(always)]
pub(super) fn restore_pc_is_elr() {
    unsafe {
        asm!(
            "
    ldp X0, X10,    [sp], #0x10
    msr	ELR_EL1,    X10
        "
        );
    }
    __ctx_restore_x_q();
}

#[inline(always)]
fn __ctx_restore_x_q() {
    unsafe {
        #[cfg(hard_float)]
        asm!(
            "
    ldp    x9,  x10, [sp], #0x10
    msr    fpcr, x9
    msr    fpsr, x10
    ldp    q0,  q1,  [sp], #0x20
    ldp    q2,  q3,  [sp], #0x20
    ldp    q4,  q5,  [sp], #0x20
    ldp    q6,  q7,  [sp], #0x20
    ldp    q8,  q9,  [sp], #0x20
    ldp    q10, q11, [sp], #0x20
    ldp    q12, q13, [sp], #0x20
    ldp    q14, q15, [sp], #0x20
    ldp    q16, q17, [sp], #0x20
    ldp    q18, q19, [sp], #0x20
    ldp    q20, q21, [sp], #0x20
    ldp    q22, q23, [sp], #0x20
    ldp    q24, q25, [sp], #0x20
    ldp    q26, q27, [sp], #0x20
    ldp    q28, q29, [sp], #0x20
    ldp    q30, q31, [sp], #0x20
            "
        );

        asm!(
            "
    ldp X9,X0,      [sp], #0x10
    msr	SPSR_EL1,   X9
	ldp	X1,X2,      [sp], #0x10
    ldp X3,X4,      [sp], #0x10
	ldp X5,X6,      [sp], #0x10
	ldp	X7,X8,      [sp], #0x10
	ldp	X9,X10,     [sp], #0x10
	ldp	X11,X12,    [sp], #0x10
	ldp	X13,X14,    [sp], #0x10
	ldp	X15,X16,    [sp], #0x10
	ldp	X17,X18,    [sp], #0x10
	ldp	X19,x20,    [sp], #0x10
	ldp	X21,X22,    [sp], #0x10
	ldp	X23,X24,    [sp], #0x10
	ldp	X25,X26,    [sp], #0x10
	ldp	X27,X28,    [sp], #0x10
	ldp	X29,X30,    [sp], #0x10
        "
        );
    }
}

#[inline(never)]
#[unsafe(no_mangle)]
pub fn tcb_switch(prev_ptr: *mut u8, next_ptr: *mut u8) {
    store_pc_is_lr();

    unsafe {
        let mut prev = TaskControlBlock::from(prev_ptr);

        let next = TaskControlBlock::from(next_ptr);

        let sp: usize;

        asm!("mov {0}, sp", out(reg) sp);

        prev.sp = sp;
        let ctx = &mut *(prev.sp as *mut Context);
        ctx.pc = ctx.lr;

        asm!("mov sp, {0}", in(reg) next.sp);
    }

    restore_pc_is_lr();
}
