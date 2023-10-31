pub mod legacy;

use std::cell::Ref;

use crate::{state, types::Timestamp};
use anchor_lang::prelude::{
    error, require, require_keys_eq, AccountInfo, ErrorCode, Pubkey, Result,
};

pub(self) const ETH_PUBKEY_SIZE: usize = 20;

pub enum GuardianSetAccount<'a> {
    Account(GuardianSet<'a>),
    LegacyAccount(legacy::GuardianSet<'a>),
}

impl<'a> GuardianSetAccount<'a> {
    /// Index representing an incrementing version number for this guardian set.
    pub fn index(&self) -> u32 {
        match self {
            Self::Account(inner) => inner.index(),
            Self::LegacyAccount(inner) => inner.index(),
        }
    }

    /// Number of guardians in set.
    pub fn num_guardians(&self) -> usize {
        match self {
            Self::Account(inner) => inner.num_guardians(),
            Self::LegacyAccount(inner) => inner.num_guardians(),
        }
    }

    /// Ethereum-style public key.
    pub fn key(&self, i: usize) -> [u8; ETH_PUBKEY_SIZE] {
        match self {
            Self::Account(inner) => inner.key(i),
            Self::LegacyAccount(inner) => inner.key(i),
        }
    }

    pub fn creation_time(&self) -> Timestamp {
        match self {
            Self::Account(inner) => inner.creation_time(),
            Self::LegacyAccount(inner) => inner.creation_time(),
        }
    }

    pub fn expiration_time(&self) -> Timestamp {
        match self {
            Self::Account(inner) => inner.expiration_time(),
            Self::LegacyAccount(inner) => inner.expiration_time(),
        }
    }

    pub fn is_active(&self, timestamp: &Timestamp) -> bool {
        match self {
            Self::Account(inner) => inner.is_active(timestamp),
            Self::LegacyAccount(inner) => inner.is_active(timestamp),
        }
    }

    pub fn account(&'a self) -> Option<&'a GuardianSet<'a>> {
        match self {
            Self::Account(inner) => Some(inner),
            _ => None,
        }
    }

    pub fn legacy_account(&'a self) -> Option<&'a legacy::GuardianSet<'a>> {
        match self {
            Self::LegacyAccount(inner) => Some(inner),
            _ => None,
        }
    }
}

impl<'a> crate::zero_copy::LoadZeroCopy<'a> for GuardianSetAccount<'a> {
    fn load(acc_info: &'a AccountInfo) -> Result<Self> {
        use anchor_lang::Discriminator;

        require_keys_eq!(*acc_info.owner, crate::ID, ErrorCode::ConstraintOwner);

        // For legacy guardian sets, there is no discriminator. This conditional is safe to do
        // because the byte configuration for this discriminator will never equal a real index
        // (byte slice [0..4]) and number of guardian keys (byte slice [4..8]).
        let possible_discriminator = {
            let data = acc_info.try_borrow_data()?;
            require!(data.len() > 8, ErrorCode::AccountDidNotDeserialize);

            data[..8].try_into().unwrap()
        };

        let parsed = match possible_discriminator {
            state::GuardianSet::DISCRIMINATOR => Self::Account(GuardianSet::new(acc_info)?),
            _ => Self::LegacyAccount(legacy::GuardianSet::new(acc_info)?),
        };

        // Re-derive PDA address.
        let (expected_address, _) = Pubkey::find_program_address(
            &[
                state::GuardianSet::SEED_PREFIX,
                &parsed.index().to_be_bytes(),
            ],
            &crate::ID,
        );
        require_keys_eq!(*acc_info.key, expected_address, ErrorCode::ConstraintSeeds);

        Ok(parsed)
    }
}

pub struct GuardianSet<'a>(Ref<'a, &'a mut [u8]>);

impl<'a> GuardianSet<'a> {
    /// Index representing an incrementing version number for this guardian set.
    pub fn index(&self) -> u32 {
        u32::from_le_bytes(self.0[8..12].try_into().unwrap())
    }

    /// Number of guardians in set.
    pub fn num_guardians(&self) -> usize {
        u32::from_le_bytes(self.0[12..16].try_into().unwrap())
            .try_into()
            .unwrap()
    }

    /// Ethereum-style public key.
    pub fn key(&self, i: usize) -> [u8; ETH_PUBKEY_SIZE] {
        self.0[(16 + i * ETH_PUBKEY_SIZE)..(16 + (i + 1) * ETH_PUBKEY_SIZE)]
            .try_into()
            .unwrap()
    }

    /// Timestamp representing the time this guardian became active.
    pub fn creation_time(&self) -> Timestamp {
        u32::from_le_bytes(
            self.0[(16 + self.num_guardians() * ETH_PUBKEY_SIZE)
                ..(20 + self.num_guardians() * ETH_PUBKEY_SIZE)]
                .try_into()
                .unwrap(),
        )
        .into()
    }

    /// Expiration time when VAAs issued by this set are no longer valid.
    pub fn expiration_time(&self) -> Timestamp {
        u32::from_le_bytes(
            self.0[(20 + self.num_guardians() * ETH_PUBKEY_SIZE)
                ..(24 + self.num_guardians() * ETH_PUBKEY_SIZE)]
                .try_into()
                .unwrap(),
        )
        .into()
    }

    pub fn is_active(&self, timestamp: &Timestamp) -> bool {
        let expiry = self.expiration_time();
        expiry == 0 || expiry >= *timestamp
    }

    fn new(acc_info: &'a AccountInfo) -> Result<Self> {
        // The discriminator and owner of this account will have been checked by this point, so this
        // method is infallible.
        acc_info.try_borrow_data().map(Self).map_err(Into::into)
    }
}
