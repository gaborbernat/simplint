// The manylinux startup objects carry a build note for code the linker discards.
core::arch::global_asm!(
    ".pushsection .text.unused,\"ax\",@progbits",
    ".Lunused:",
    ".byte 0",
    ".popsection",
    ".pushsection .gnu.build.attributes,\"\",@note",
    ".quad .Lunused",
    ".popsection",
);

fn main() {}
