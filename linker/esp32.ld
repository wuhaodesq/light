/* ESP32 Linker Script */

MEMORY
{
    FLASH (rx)  : ORIGIN = 0x00000000, LENGTH = 4M
    SRAM (rwx)  : ORIGIN = 0x3FF00000, LENGTH = 200K
    DRAM (rwx)  : ORIGIN = 0x3FF80000, LENGTH = 200K
}

ENTRY(Reset_Handler)

SECTIONS
{
    .text :
    {
        . = ALIGN(4);
        _stext = .;
        *(.literal .text .literal.* .text.*)
        _etext = .;
    } > FLASH

    .rodata :
    {
        . = ALIGN(4);
        *(.rodata)
        *(.rodata*)
        _erodata = .;
    } > FLASH

    .data :
    {
        . = ALIGN(4);
        _sdata = .;
        *(.data)
        *(.data*)
        _edata = .;
    } > DRAM AT > FLASH

    .bss :
    {
        . = ALIGN(4);
        _sbss = .;
        *(.bss)
        *(.bss*)
        *(COMMON)
        _ebss = .;
    } > DRAM

    _end = .;
}
