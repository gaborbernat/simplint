/* The manylinux startup objects have build notes whose relocations point to discarded code. */
SECTIONS { /DISCARD/ : { *(.gnu.build.attributes*) } } INSERT AFTER .text;
