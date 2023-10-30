use std::cell::Ref;

use crate::types::Timestamp;
use anchor_lang::prelude::{error, require, require_eq, AccountInfo, ErrorCode, Result};

use super::ETH_PUBKEY_SIZE;

/// Account used to store the current configuration of the bridge, including tracking Wormhole fee
/// payments. For governance decrees, the guardian set index is used to determine whether a decree
/// was attested for using the latest guardian set.
pub struct GuardianSet<'a>(Ref<'a, &'a mut [u8]>);

impl<'a> GuardianSet<'a> {
    /// Index representing an incrementing version number for this guardian set.
    pub fn index(&self) -> u32 {
        u32::from_le_bytes(self.0[..4].try_into().unwrap())
    }

    /// Number of guardians in set.
    pub fn num_guardians(&self) -> usize {
        u32::from_le_bytes(self.0[4..8].try_into().unwrap())
            .try_into()
            .unwrap()
    }

    /// Ethereum-style public key.
    pub fn key(&self, i: usize) -> [u8; ETH_PUBKEY_SIZE] {
        self.0[(8 + i * ETH_PUBKEY_SIZE)..(8 + (i + 1) * ETH_PUBKEY_SIZE)]
            .try_into()
            .unwrap()
    }

    /// Timestamp representing the time this guardian became active.
    pub fn creation_time(&self) -> Timestamp {
        u32::from_le_bytes(
            self.0[(8 + self.num_guardians() * ETH_PUBKEY_SIZE)
                ..(12 + self.num_guardians() * ETH_PUBKEY_SIZE)]
                .try_into()
                .unwrap(),
        )
        .into()
    }

    /// Expiration time when VAAs issued by this set are no longer valid.
    pub fn expiration_time(&self) -> Timestamp {
        u32::from_le_bytes(
            self.0[(12 + self.num_guardians() * ETH_PUBKEY_SIZE)
                ..(16 + self.num_guardians() * ETH_PUBKEY_SIZE)]
                .try_into()
                .unwrap(),
        )
        .into()
    }

    pub fn is_active(&self, timestamp: &Timestamp) -> bool {
        // Note: This is a fix for Wormhole on mainnet.  The initial guardian set was never expired
        // so we block it here.
        if self.index() == 0 && self.creation_time() == 1628099186 {
            false
        } else {
            let expiry = self.expiration_time();
            expiry == 0 || expiry >= *timestamp
        }
    }

    pub(super) fn new(acc_info: &'a AccountInfo) -> Result<Self> {
        let data = acc_info.try_borrow_data()?;

        // There must be at least one guardian, which means the encoded key length is at least 1 and
        // remanining bytes in the account are 16.
        require!(data.len() > 16, ErrorCode::AccountDidNotDeserialize);

        let parsed = Self(data);
        require_eq!(
            parsed.0.len(),
            16 + parsed.num_guardians() * ETH_PUBKEY_SIZE,
            ErrorCode::AccountDidNotDeserialize
        );

        Ok(parsed)
    }
}
