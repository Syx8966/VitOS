#[cfg(feature = "axstd")]
use axstd::println;

const EI_CLASS: usize = 4;
const EI_DATA: usize = 5;
const ELFCLASS64: u8 = 2;
const ELFDATA2LSB: u8 = 1;
const ET_EXEC: u16 = 2;
const EM_RISCV: u16 = 243;
const EM_LOONGARCH: u16 = 258;
const PT_LOAD: u32 = 1;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ElfError {
    BufferTooSmall,
    BadMagic,
    UnsupportedClass,
    UnsupportedEndian,
    UnsupportedType,
    UnsupportedMachine,
    InvalidProgramHeaderTable,
    InvalidProgramHeader,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ElfHeader {
    pub machine: u16,
    pub entry: u64,
    pub phoff: u64,
    pub phentsize: u16,
    pub phnum: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoadSegment {
    pub offset: u64,
    pub vaddr: u64,
    pub filesz: u64,
    pub memsz: u64,
    pub flags: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParsedElf {
    pub header: ElfHeader,
    pub load_segments: [Option<LoadSegment>; 8],
    pub load_segment_count: usize,
}

impl ParsedElf {
    pub fn load_segments(&self) -> impl Iterator<Item = LoadSegment> + '_ {
        self.load_segments.iter().flatten().copied()
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> Result<u16, ElfError> {
    let value = bytes
        .get(offset..offset + 2)
        .ok_or(ElfError::BufferTooSmall)?;
    Ok(u16::from_le_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> Result<u32, ElfError> {
    let value = bytes
        .get(offset..offset + 4)
        .ok_or(ElfError::BufferTooSmall)?;
    Ok(u32::from_le_bytes([value[0], value[1], value[2], value[3]]))
}

fn read_u64(bytes: &[u8], offset: usize) -> Result<u64, ElfError> {
    let value = bytes
        .get(offset..offset + 8)
        .ok_or(ElfError::BufferTooSmall)?;
    Ok(u64::from_le_bytes([
        value[0], value[1], value[2], value[3], value[4], value[5], value[6], value[7],
    ]))
}

pub fn parse(bytes: &[u8]) -> Result<ParsedElf, ElfError> {
    if bytes.len() < 64 {
        return Err(ElfError::BufferTooSmall);
    }
    if bytes.get(0..4) != Some(b"\x7fELF") {
        return Err(ElfError::BadMagic);
    }
    if bytes[EI_CLASS] != ELFCLASS64 {
        return Err(ElfError::UnsupportedClass);
    }
    if bytes[EI_DATA] != ELFDATA2LSB {
        return Err(ElfError::UnsupportedEndian);
    }

    let elf_type = read_u16(bytes, 16)?;
    if elf_type != ET_EXEC {
        return Err(ElfError::UnsupportedType);
    }

    let machine = read_u16(bytes, 18)?;
    if !matches!(machine, EM_RISCV | EM_LOONGARCH) {
        return Err(ElfError::UnsupportedMachine);
    }

    let header = ElfHeader {
        machine,
        entry: read_u64(bytes, 24)?,
        phoff: read_u64(bytes, 32)?,
        phentsize: read_u16(bytes, 54)?,
        phnum: read_u16(bytes, 56)?,
    };

    if header.phentsize < 56 {
        return Err(ElfError::InvalidProgramHeaderTable);
    }

    let phoff = usize::try_from(header.phoff).map_err(|_| ElfError::InvalidProgramHeaderTable)?;
    let phentsize = usize::from(header.phentsize);
    let phnum = usize::from(header.phnum);
    let table_size = phentsize
        .checked_mul(phnum)
        .ok_or(ElfError::InvalidProgramHeaderTable)?;
    if phoff
        .checked_add(table_size)
        .filter(|end| *end <= bytes.len())
        .is_none()
    {
        return Err(ElfError::InvalidProgramHeaderTable);
    }

    let mut load_segments = [None; 8];
    let mut load_segment_count = 0;
    for index in 0..phnum {
        let offset = phoff + index * phentsize;
        let ph_type = read_u32(bytes, offset)?;
        if ph_type != PT_LOAD {
            continue;
        }
        if load_segment_count == load_segments.len() {
            return Err(ElfError::InvalidProgramHeaderTable);
        }

        let segment = LoadSegment {
            flags: read_u32(bytes, offset + 4)?,
            offset: read_u64(bytes, offset + 8)?,
            vaddr: read_u64(bytes, offset + 16)?,
            filesz: read_u64(bytes, offset + 32)?,
            memsz: read_u64(bytes, offset + 40)?,
        };
        if segment.filesz > segment.memsz {
            return Err(ElfError::InvalidProgramHeader);
        }
        load_segments[load_segment_count] = Some(segment);
        load_segment_count += 1;
    }

    Ok(ParsedElf {
        header,
        load_segments,
        load_segment_count,
    })
}

pub fn embedded_user_hello_for_current_arch() -> &'static [u8] {
    match option_env!("VITOS_BOOT_ARCH") {
        Some("loongarch64") => embedded_user_hello_la(),
        _ => embedded_user_hello_rv(),
    }
}

pub fn embedded_local_basic_for_current_arch() -> &'static [u8] {
    match option_env!("VITOS_BOOT_ARCH") {
        Some("loongarch64") => embedded_local_basic_la(),
        _ => embedded_local_basic_rv(),
    }
}

pub fn smoke_test() -> Result<ParsedElf, ElfError> {
    let parsed = parse(embedded_user_hello_for_current_arch())?;

    #[cfg(feature = "axstd")]
    {
        println!(
            "[elf] entry=0x{:x} phnum={} load_segments={}",
            parsed.header.entry, parsed.header.phnum, parsed.load_segment_count
        );
        for (index, segment) in parsed.load_segments().enumerate() {
            println!(
                "[elf] load[{}] off=0x{:x} vaddr=0x{:x} filesz=0x{:x} memsz=0x{:x} flags=0x{:x}",
                index, segment.offset, segment.vaddr, segment.filesz, segment.memsz, segment.flags
            );
        }
    }

    Ok(parsed)
}

pub fn embedded_user_hello_rv() -> &'static [u8] {
    include_bytes!(env!("VITOS_USER_HELLO_RV"))
}

pub fn embedded_user_hello_la() -> &'static [u8] {
    include_bytes!(env!("VITOS_USER_HELLO_LA"))
}

pub fn embedded_local_basic_rv() -> &'static [u8] {
    include_bytes!(env!("VITOS_LOCAL_BASIC_RV"))
}

pub fn embedded_local_basic_la() -> &'static [u8] {
    include_bytes!(env!("VITOS_LOCAL_BASIC_LA"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_embedded_riscv64_hello_elf() {
        let parsed = parse(embedded_user_hello_rv()).expect("RISC-V64 hello ELF should parse");

        assert_eq!(parsed.header.machine, EM_RISCV);
        assert_eq!(parsed.header.entry, 0x400000);
        assert!(parsed.header.phnum >= 1);
        assert!(parsed.load_segment_count >= 1);

        let segment = parsed.load_segments().next().expect("LOAD segment");
        assert_eq!(segment.vaddr, 0x400000);
        assert!(segment.filesz <= segment.memsz);
        assert_eq!(segment.flags, 0x5);
    }

    #[test]
    fn parses_embedded_loongarch64_hello_elf() {
        let parsed = parse(embedded_user_hello_la()).expect("LoongArch64 hello ELF should parse");

        assert_eq!(parsed.header.machine, EM_LOONGARCH);
        assert_eq!(parsed.header.entry, 0x400000);
        assert!(parsed.header.phnum >= 1);
        assert!(parsed.load_segment_count >= 1);

        let segment = parsed.load_segments().next().expect("LOAD segment");
        assert_eq!(segment.vaddr, 0x400000);
        assert!(segment.filesz <= segment.memsz);
        assert_eq!(segment.flags, 0x5);
    }

    #[test]
    fn parses_embedded_local_basic_elves() {
        let rv = parse(embedded_local_basic_rv()).expect("RISC-V64 local-basic ELF should parse");
        let la =
            parse(embedded_local_basic_la()).expect("LoongArch64 local-basic ELF should parse");

        assert_eq!(rv.header.machine, EM_RISCV);
        assert_eq!(la.header.machine, EM_LOONGARCH);
        assert!(rv.load_segment_count >= 1);
        assert!(la.load_segment_count >= 1);
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = embedded_user_hello_rv().to_vec();
        bytes[0] = 0;

        assert_eq!(parse(&bytes), Err(ElfError::BadMagic));
    }

    #[test]
    fn rejects_segment_larger_than_memory() {
        let mut bytes = embedded_user_hello_rv().to_vec();
        let filesz_offset = 64 + 32;
        let memsz_offset = 64 + 40;
        bytes[filesz_offset..filesz_offset + 8].copy_from_slice(&0x2000_u64.to_le_bytes());
        bytes[memsz_offset..memsz_offset + 8].copy_from_slice(&0x1000_u64.to_le_bytes());

        assert_eq!(parse(&bytes), Err(ElfError::InvalidProgramHeader));
    }
}
