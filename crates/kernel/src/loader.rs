#[cfg(feature = "axstd")]
use axstd::println;

use crate::elf;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoaderStatus {
    ReadyForStaticElf,
    ParsedStaticElf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoaderState {
    pub status: LoaderStatus,
    pub user_entry: usize,
    pub user_stack_top: usize,
    pub load_segment_count: usize,
}

impl LoaderState {
    pub const fn ready_for_static_elf() -> Self {
        Self {
            status: LoaderStatus::ReadyForStaticElf,
            user_entry: 0,
            user_stack_top: 0,
            load_segment_count: 0,
        }
    }

    pub const fn parsed_static_elf(user_entry: usize, load_segment_count: usize) -> Self {
        Self {
            status: LoaderStatus::ParsedStaticElf,
            user_entry,
            user_stack_top: 0,
            load_segment_count,
        }
    }
}

pub fn init() -> LoaderState {
    #[cfg(feature = "axstd")]
    {
        println!("[loader] status = {:?}", LoaderStatus::ReadyForStaticElf);
        println!("[loader] next = parse static ELF headers");
    }

    match elf::smoke_test() {
        Ok(parsed) => {
            let state = LoaderState::parsed_static_elf(
                parsed.header.entry as usize,
                parsed.load_segment_count,
            );
            #[cfg(feature = "axstd")]
            println!(
                "[loader] status = {:?} entry=0x{:x} load_segments={}",
                state.status, state.user_entry, state.load_segment_count
            );
            state
        }
        Err(_err) => {
            #[cfg(feature = "axstd")]
            println!("[loader] ELF smoke test failed: {:?}", _err);
            LoaderState::ready_for_static_elf()
        }
    }
}
