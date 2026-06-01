/* Snake SoC v0.1 — Model B linker script (Rust runtime).
 *
 * Single MEMORY region: all code and data live in DRAM at 0x8000_0000. No
 * LMA/VMA split — Model B's ELF loader places every PT_LOAD at its p_paddr
 * inside DRAM, then crt0 zeroes .bss (the loader does NOT zero
 * p_memsz - p_filesz; that is crt0's job, Snake SoC spec §5.3 step 4).
 *
 * Reset PC is 0x0 (Boot ROM); the 8-byte ROM stub jumps here. e_entry must be
 * 0x8000_0000, so crt0 (`.text.init`) is KEEP'd first in .text.
 *
 * Symbols consumed by the runtime:
 *   __global_pointer$ — gp base (crt0 `la gp, ...` under .option norelax)
 *   __stack_top       — initial sp (top of the 64 KiB stack region)
 *   _sbss / _ebss     — .bss bounds (crt0 zero-fill, 4-byte aligned)
 *   _sheap / _eheap   — bump-allocator heap bounds
 */

OUTPUT_ARCH(riscv)
ENTRY(_start)

MEMORY
{
    DRAM (rwx) : ORIGIN = 0x80000000, LENGTH = 64M
}

__heap_size  = 0x10000;  /* 64 KiB */
__stack_size = 0x10000;  /* 64 KiB */

SECTIONS
{
    .text : ALIGN(4)
    {
        /* crt0 must sit at e_entry (0x8000_0000). Keep .text.init first and
         * never let the linker garbage-collect or reorder it. */
        KEEP(*(.text.init));
        *(.text .text.*);
    } > DRAM

    .rodata : ALIGN(4)
    {
        *(.rodata .rodata.*);
        *(.srodata .srodata.*);
    } > DRAM

    .data : ALIGN(4)
    {
        PROVIDE(_sdata = .);
        /* gp points 0x800 into .data/.sdata so the ±2 KiB gp-relative window
         * straddles the small-data region. */
        PROVIDE(__global_pointer$ = . + 0x800);
        *(.sdata .sdata.*);
        *(.data .data.*);
        PROVIDE(_edata = .);
    } > DRAM

    .bss (NOLOAD) : ALIGN(4)
    {
        PROVIDE(_sbss = .);
        *(.sbss .sbss.*);
        *(.bss .bss.*);
        *(COMMON);
        . = ALIGN(4);
        PROVIDE(_ebss = .);
    } > DRAM

    .heap (NOLOAD) : ALIGN(16)
    {
        PROVIDE(_sheap = .);
        . += __heap_size;
        PROVIDE(_eheap = .);
    } > DRAM

    .stack (NOLOAD) : ALIGN(16)
    {
        PROVIDE(_sstack = .);
        . += __stack_size;
        PROVIDE(__stack_top = .);
    } > DRAM

    /DISCARD/ :
    {
        *(.eh_frame .eh_frame_hdr);
        *(.comment);
        *(.debug*);
    }
}
