pub mod resume;

use std::{
	collections::HashMap,
	num::NonZero,
	sync::{Arc, RwLock},
};

use gdbstub::stub::MultiThreadStopReason;
use nix::sys::pthread::Pthread;

use crate::{
	gdb::resume::{ResumeMarker, ResumeMode},
	vm::{
		KernelInfo, VirtualizationBackend, VmPeripherals, internal::VirtualizationBackendInternal,
	},
};

/// A way of sending pthread IDs reliably across threads.
///
/// # Platform-specific behavior
///
/// This is particularly necessary for musl, as `Pthread` is eq
/// which can't be passed to thread as easily
///
/// # Safety
///
/// This can be safely sent across threads because pthread IDs
/// and thread-safety is ensured by the pthread library.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PthreadWrapper(pub Pthread);

unsafe impl Send for PthreadWrapper {}
unsafe impl Sync for PthreadWrapper {}

pub(crate) struct VcpuWrapperShared<VCpu> {
	pub(crate) vcpu: RwLock<VCpu>,
	pub(crate) resume: ResumeMarker,
}

pub(crate) struct VcpuWrapper<VCpu> {
	pub(crate) shared: Arc<VcpuWrapperShared<VCpu>>,

	pub(crate) pthread: PthreadWrapper,
	/// This does look odd, but GDB appears to truncate thread-ids to 32bit
	pub(crate) tid: NonZero<u32>,

	pub(crate) planned_resume_mode: Option<ResumeMode>,
}

pub(crate) struct Freewheel<Vm: VirtualizationBackend> {
	#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
	pub(crate) breakpoints: Arc<RwLock<crate::linux::gdb::breakpoints::AllBreakpoints>>,

	pub(crate) peripherals: Arc<VmPeripherals>,
	pub(crate) kernel_info: Arc<KernelInfo>,
	pub(crate) stops: async_channel::Receiver<MultiThreadStopReason<u64>>,
	pub(crate) vcpus: Vec<VcpuWrapper<<Vm::BACKEND as VirtualizationBackendInternal>::VCPU>>,
	/// This does look odd, but GDB appears to truncate thread-ids to 32bit
	pub(crate) tid_to_vcpu: HashMap<NonZero<u32>, usize>,

	pub(crate) is_initializing: bool,
	pub(crate) default_resume_mode: ResumeMode,
}
