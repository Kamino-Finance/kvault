//! Zero-copy account deserialization for on-chain Kvault accounts.
//!
//! All account types use `bytemuck` for zero-copy access and
//! `SplDiscriminate` for 8-byte Anchor discriminator validation.
//! Use [`from_account_data`] to cast raw account bytes (including
//! the discriminator prefix) to a typed reference.

mod global_config;
mod pod;
mod reserve_whitelist;
mod vault_allocation;
mod vault_reward_info;
mod vault_state;

pub use global_config::*;
pub use pod::PodU128;
pub use reserve_whitelist::*;
pub use spl_discriminator::{ArrayDiscriminator, SplDiscriminate};
pub use vault_allocation::*;
pub use vault_reward_info::*;
pub use vault_state::*;

/// Size of the Anchor account discriminator (8 bytes).
pub const DISCRIMINATOR_SIZE: usize = ArrayDiscriminator::LENGTH;

/// Errors returned by [`from_account_data`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AccountDataError {
    /// Account data is shorter than discriminator + struct size.
    DataTooShort { expected: usize, actual: usize },
    /// The 8-byte discriminator does not match the expected value.
    InvalidDiscriminator { expected: [u8; 8], actual: [u8; 8] },
}

impl core::fmt::Display for AccountDataError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::DataTooShort { expected, actual } => {
                write!(
                    f,
                    "account data too short: expected {expected}, got {actual}"
                )
            }
            Self::InvalidDiscriminator { expected, actual } => {
                write!(
                    f,
                    "invalid discriminator: expected {expected:?}, got {actual:?}"
                )
            }
        }
    }
}

impl std::error::Error for AccountDataError {}

/// Cast raw account data (including the 8-byte Anchor discriminator) to `&T`.
///
/// Verifies the discriminator matches `T::SPL_DISCRIMINATOR` before casting.
pub fn from_account_data<T: bytemuck::Pod + SplDiscriminate>(
    data: &[u8],
) -> Result<&T, AccountDataError> {
    let expected_len = DISCRIMINATOR_SIZE + core::mem::size_of::<T>();
    if data.len() < expected_len {
        return Err(AccountDataError::DataTooShort {
            expected: expected_len,
            actual: data.len(),
        });
    }
    let disc = &data[..DISCRIMINATOR_SIZE];
    if disc != T::SPL_DISCRIMINATOR_SLICE {
        let mut actual = [0u8; 8];
        actual.copy_from_slice(disc);
        let mut expected = [0u8; 8];
        expected.copy_from_slice(T::SPL_DISCRIMINATOR_SLICE);
        return Err(AccountDataError::InvalidDiscriminator { expected, actual });
    }
    Ok(bytemuck::from_bytes(
        &data[DISCRIMINATOR_SIZE..expected_len],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verify_account_sizes() {
        assert_eq!(core::mem::size_of::<VaultState>(), 62544);
        assert_eq!(core::mem::size_of::<VaultAllocation>(), 2160);
        assert_eq!(core::mem::size_of::<GlobalConfig>(), 1024);
        assert_eq!(core::mem::size_of::<ReserveWhitelistEntry>(), 128);
    }

    #[test]
    fn verify_account_discriminators() {
        use sha2::{Digest, Sha256};

        macro_rules! check {
            ($ty:ty, $name:expr) => {{
                let mut h = Sha256::new();
                h.update(concat!("account:", $name).as_bytes());
                let hash = h.finalize();
                let mut expected = [0u8; 8];
                expected.copy_from_slice(&hash[..8]);
                assert_eq!(
                    <$ty as SplDiscriminate>::SPL_DISCRIMINATOR_SLICE,
                    &expected,
                    concat!("Discriminator mismatch for ", $name),
                );
            }};
        }

        check!(VaultState, "VaultState");
        check!(GlobalConfig, "GlobalConfig");
    }
}
