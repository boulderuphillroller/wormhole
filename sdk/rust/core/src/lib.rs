//! Core acts as a provider for all the Wormhole primitives. These are utilised within every single
//! Wormhole Rust project.

#![deny(unused_results)]

use borsh::{
    BorshDeserialize,
    BorshSerialize,
};
pub use {
    chain::*,
    error::*,
    vaa::*,
};

pub mod chain;
pub mod vaa;

#[macro_use]
pub mod error;


pub const GOVERNANCE_EMITTER: [u8; 32] =
    hex_literal::hex!("0000000000000000000000000000000000000000000000000000000000000004");


#[derive(BorshDeserialize, BorshSerialize, Default)]
pub struct GuardianSet {
    pub index:     u32,
    pub expires:   u32,
    pub addresses: Vec<[u8; 20]>,
}

impl GuardianSet {
    pub fn quorum(&self) -> usize {
        ((self.addresses.len() * 10 / 3) * 2) / 10 + 1
    }
}


/// Helper method that attempts to parse and truncate UTF-8 from a byte stream. This is useful when
/// the wire data is expected to contain UTF-8 that is either already truncated, or needs to be,
/// while still maintaining the ability to render.
///
/// This should be used to parse any Text-over-Wormhole fields that are meant to be human readable.
pub(crate) fn parse_fixed_utf8<T: AsRef<[u8]>, const N: usize>(s: T) -> Option<String> {
    use {
        bstr::ByteSlice,
        std::io::{
            Cursor,
            Read,
        },
    };

    // Read Bytes.
    let mut cursor = Cursor::new(s.as_ref());
    let mut buffer = vec![0u8; N];
    cursor.read_exact(&mut buffer).ok()?;
    buffer.retain(|&c| c != 0);

    // Attempt UTF-8 Decoding. Stripping invalid Unicode characters (0xFFFD).
    let mut buffer: Vec<char> = buffer.chars().collect();
    buffer.retain(|&c| c != '\u{FFFD}');

    Some(buffer.iter().collect())
}
