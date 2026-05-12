#[cfg(feature = "axstd")]
use axstd::println;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LoaderStatus {
    ReadyForStaticElf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LoaderState {
    pub status: LoaderStatus,
    pub user_entry: usize,
    pub user_stack_top: usize,
}

impl LoaderState {
    pub const fn ready_for_static_elf() -> Self {
        Self {
            status: LoaderStatus::ReadyForStaticElf,
            user_entry: 0,
            user_stack_top: 0,
        }
    }
}

pub fn init() -> LoaderState {
    let state = LoaderState::ready_for_static_elf();

    #[cfg(feature = "axstd")]
    {
        println!("[loader] status = {:?}", state.status);
        println!("[loader] next = parse static ELF headers");
    }

    state
}
