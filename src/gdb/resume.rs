use std::sync::atomic::AtomicU8;

use event_listener::Event;

pub(crate) struct ResumeMarker {
	pub(crate) mode: AtomicU8,
	pub(crate) event: Event,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ResumeMode {
	/// The vCPU is stopped (r#continue won't get called while this is set)
	Stopped,
	/// The vCPU is single-stepped
	Step,
	/// The vCPU is uninterrupt-runnable
	Freewheel,
}
