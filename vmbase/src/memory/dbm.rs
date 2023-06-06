// Copyright 2023, The Android Open Source Project
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

//! Hardware management of the access flag and dirty state.

use super::page_table::is_leaf_pte;
use super::util::flush_region;
use crate::{isb, read_sysreg, write_sysreg};
use aarch64_paging::paging::{Attributes, Descriptor, MemoryRegion};

/// Sets whether the hardware management of access and dirty state is enabled with
/// the given boolean.
pub fn set_dbm_enabled(enabled: bool) {
    if !dbm_available() {
        return;
    }
    // TCR_EL1.{HA,HD} bits controlling hardware management of access and dirty state
    const TCR_EL1_HA_HD_BITS: usize = 3 << 39;

    let mut tcr = read_sysreg!("tcr_el1");
    if enabled {
        tcr |= TCR_EL1_HA_HD_BITS
    } else {
        tcr &= !TCR_EL1_HA_HD_BITS
    };
    // Safe because it writes to a system register and does not affect Rust.
    unsafe { write_sysreg!("tcr_el1", tcr) }
    isb!();
}

/// Returns `true` if hardware dirty state management is available.
fn dbm_available() -> bool {
    if !cfg!(feature = "cpu_feat_hafdbs") {
        return false;
    }
    // Hardware dirty bit management available flag (ID_AA64MMFR1_EL1.HAFDBS[1])
    const DBM_AVAILABLE: usize = 1 << 1;
    read_sysreg!("id_aa64mmfr1_el1") & DBM_AVAILABLE != 0
}

/// Flushes a memory range the descriptor refers to, if the descriptor is in writable-dirty state.
/// As the return type is required by the crate `aarch64_paging`, we cannot address the lint
/// issue `clippy::result_unit_err`.
#[allow(clippy::result_unit_err)]
pub fn flush_dirty_range(
    va_range: &MemoryRegion,
    desc: &mut Descriptor,
    level: usize,
) -> Result<(), ()> {
    // Only flush ranges corresponding to dirty leaf PTEs.
    let flags = desc.flags().ok_or(())?;
    if !is_leaf_pte(&flags, level) {
        return Ok(());
    }
    if !flags.contains(Attributes::READ_ONLY) {
        flush_region(va_range.start().0, va_range.len());
    }
    Ok(())
}