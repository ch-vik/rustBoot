MEMORY
{
  /* NOTE 1 K = 1 KiBi = 1024 bytes */
  /* ATSAMD21J17A has 128KB Flash and 16KB RAM */
  /* Reserve 32KB for bootloader + 0x100 (256 byte) offset for rustBoot header */
  FLASH    (rx)  : ORIGIN = 0x8100, LENGTH = 32K
  RAM      (rwx) : ORIGIN = 0x20000000, LENGTH = 16K
}