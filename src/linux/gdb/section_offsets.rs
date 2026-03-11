use gdbstub::target::{self, ext::section_offsets::Offsets};

use super::Freewheel;
use crate::vm::VirtualizationBackend;

impl<Vm: VirtualizationBackend> target::ext::section_offsets::SectionOffsets for Freewheel<Vm> {
	fn get_section_offsets(&mut self) -> Result<Offsets<u64>, Self::Error> {
		let offset = self.kernel_info.kernel_address.as_u64();
		Ok(Offsets::Sections {
			text: offset,
			data: offset,
			bss: Some(offset),
		})
	}
}
