ENTRY(_start)

PHDRS
{
  text PT_LOAD FLAGS(5);
  rodata PT_LOAD FLAGS(4);
}

SECTIONS
{
  . = 0x400000;

  .text : ALIGN(4K)
  {
    *(.text.entry)
    *(.text .text.*)
  } :text

  .rodata : ALIGN(4K)
  {
    *(.rodata .rodata.*)
  } :rodata

  /DISCARD/ :
  {
    *(.comment)
    *(.eh_frame)
    *(.debug*)
  }
}
