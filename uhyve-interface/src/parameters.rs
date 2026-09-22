//! Parameters for hypercalls.

pub(crate) use core::marker::PhantomData;
pub use core::num::NonZero;

pub use hermit_abi::{
	O_APPEND, O_CREAT, O_DIRECTORY, O_EXCL, O_RDONLY, O_RDWR, O_TRUNC, O_WRONLY, SEEK_CUR,
	SEEK_END, SEEK_SET,
	errno::{EBADF, EEXIST, EFAULT, EINVAL, EIO, EISDIR, ENOENT, EOVERFLOW, EPERM, EROFS},
};

#[cfg(doc)]
use crate::v2::parameters::{LseekParams, MkdirParams, ReadParams, StatParams, WriteParams};

// File operations supported by Hermit and Uhyve
pub const ALLOWED_OPEN_FLAGS: i32 =
	O_RDONLY | O_WRONLY | O_RDWR | O_CREAT | O_EXCL | O_TRUNC | O_APPEND | O_DIRECTORY;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TaggedNumber<T, T2> {
	num: T,
	_phantom: PhantomData<T2>,
}

/// High-level representation of a 64-bit result, of a [`ReadParams::ret`], [`WriteParams::ret`], [`LseekParams::offset`].
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IoResult64 {
	/// INVARIANT: `Ok(x)` has `x` with value `<= i64::MAX as u64`.
	Ok(u64),
	/// INVARIANT: `Errno(x)` has `x` with value `<= -(i32::MIN + 1) as u32 + 1`.
	Errno(NonZero<u32>),
}

/// High-level representation of a 32-bit result.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IoResult32 {
	/// INVARIANT: `Ok(x)` has `x` with value `<= i32::MAX as u64`.
	Ok(u32),
	/// INVARIANT: `Errno(x)` has `x` with value `<= -(i32::MIN + 1) as u32 + 1`.
	Errno(NonZero<u32>),
}

/// High-level representatzion of an [`MkdirParams::ret`], [`StatParams::ret`].
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum TristateResult {
	/// No result. Guests should set this value before calling the hypercall.
	#[default]
	None,
	/// Data was committed on success.
	Success,
	/// Error with libc errno.
	Errno(NonZero<u32>),
}

// This is unfortunately necessary because `nonzero_internals` is an unstable feature
// which would be necessary to access the `core::num::ZeroablePrimitive` trait.
macro_rules! io_result_num_convert {
	($io_res_ty:ident, $signed_num_ty:ty, $unsigned_num_ty:ty) => {
		impl From<TaggedNumber<$signed_num_ty, $io_res_ty>> for $io_res_ty {
			fn from(value: TaggedNumber<$signed_num_ty, $io_res_ty>) -> Self {
				if value.num < 0 {
					// UNWRAP: -x != 0
					// NOTE that this intentionally can't panic even if the error value exceeds the u32 range.
					Self::Errno(NonZero::new((-value.num) as u32).unwrap())
				} else {
					Self::Ok(value.num as $unsigned_num_ty)
				}
			}
		}

		impl $io_res_ty {
			/// Attempts to convert this result value into an integer.
			pub fn try_as_num(self) -> Option<TaggedNumber<$signed_num_ty, Self>> {
				match self {
					Self::Ok(x) => x.try_into().ok(),
					// The following complication is necessary to handle the result `Some($signed_num_ty::MIN)` correctly.
					Self::Errno(x) if x.get() > ((-(i32::MIN + 1)) as u32 + 1) => None,
					Self::Errno(x) => Some(-((x.get() - 1) as $signed_num_ty) - 1),
				}
				.map(|num| TaggedNumber {
					num,
					_phantom: PhantomData,
				})
			}
		}
	};
}

io_result_num_convert!(IoResult32, i32, u32);
io_result_num_convert!(IoResult64, i64, u64);

impl From<TaggedNumber<i32, TristateResult>> for TristateResult {
	fn from(value: TaggedNumber<i32, Self>) -> Self {
		if value.num > 0 {
			Self::None
		} else if let Some(e) = NonZero::new((-value.num) as u32) {
			Self::Errno(e)
		} else {
			Self::Success
		}
	}
}

impl TristateResult {
	pub fn try_as_num(self) -> Option<TaggedNumber<i32, Self>> {
		match self {
			Self::None => Some(1),
			Self::Success => Some(0),
			Self::Errno(x) if x.get() > ((-(i32::MIN + 1)) as u32 + 1) => None,
			Self::Errno(x) => Some(-((x.get() - 1) as i32) - 1),
		}
		.map(|num| TaggedNumber {
			num,
			_phantom: PhantomData,
		})
	}
}

#[cfg(test)]
mod tests {
	use super::{IoResult32, IoResult64, NonZero, PhantomData, TaggedNumber};

	#[test]
	fn test_io_result64_errno_i64_min() {
		const MAX_ERR_NUM: u32 = (-(i32::MIN + 1)) as u32 + 1;
		assert_eq!(
			IoResult64::Errno(NonZero::new(MAX_ERR_NUM - 1).unwrap()).try_as_num(),
			Some((i32::MIN + 1) as i64).map(|num| TaggedNumber {
				num,
				_phantom: PhantomData,
			})
		);
		assert_eq!(
			IoResult64::Errno(NonZero::new(MAX_ERR_NUM).unwrap()).try_as_num(),
			Some(i32::MIN as i64).map(|num| TaggedNumber {
				num,
				_phantom: PhantomData,
			})
		);
		assert_eq!(
			IoResult64::Errno(NonZero::new(MAX_ERR_NUM + 1).unwrap()).try_as_num(),
			None
		);
	}

	#[test]
	fn test_io_result64_ok_i64_max() {
		assert_eq!(
			IoResult64::Ok(i64::MAX as u64).try_as_num(),
			Some(i64::MAX).map(|num| TaggedNumber {
				num,
				_phantom: PhantomData,
			})
		);
		assert_eq!(IoResult64::Ok(i64::MAX as u64 + 1).try_as_num(), None);
	}

	#[test]
	fn test_io_result32_errno_i32_min() {
		const MAX_ERR_NUM: u32 = (-(i32::MIN + 1)) as u32 + 1;
		assert_eq!(
			IoResult32::Errno(NonZero::new(MAX_ERR_NUM - 1).unwrap()).try_as_num(),
			Some(i32::MIN + 1).map(|num| TaggedNumber {
				num,
				_phantom: PhantomData,
			})
		);
		assert_eq!(
			IoResult32::Errno(NonZero::new(MAX_ERR_NUM).unwrap()).try_as_num(),
			Some(i32::MIN).map(|num| TaggedNumber {
				num,
				_phantom: PhantomData,
			})
		);
		assert_eq!(
			IoResult32::Errno(NonZero::new(MAX_ERR_NUM + 1).unwrap()).try_as_num(),
			None
		);
	}

	#[test]
	fn test_io_result32_ok_i32_max() {
		assert_eq!(
			IoResult32::Ok(i32::MAX as u32).try_as_num(),
			Some(i32::MAX).map(|num| TaggedNumber {
				num,
				_phantom: PhantomData,
			})
		);
		assert_eq!(IoResult32::Ok(i32::MAX as u32 + 1).try_as_num(), None);
	}

	#[test]
	fn test_tristate_result_errno_i32_min() {
		use super::TristateResult;

		const MAX_ERR_NUM: u32 = (-(i32::MIN + 1)) as u32 + 1;
		assert_eq!(
			TristateResult::Errno(NonZero::new(MAX_ERR_NUM - 1).unwrap()).try_as_num(),
			Some(i32::MIN + 1).map(|num| TaggedNumber {
				num,
				_phantom: PhantomData,
			})
		);
		assert_eq!(
			TristateResult::Errno(NonZero::new(MAX_ERR_NUM).unwrap()).try_as_num(),
			Some(i32::MIN).map(|num| TaggedNumber {
				num,
				_phantom: PhantomData,
			})
		);
		assert_eq!(
			TristateResult::Errno(NonZero::new(MAX_ERR_NUM + 1).unwrap()).try_as_num(),
			None
		);
	}
}
