use crate::{error::CoreBridgeError, legacy::utils::LegacyAnchorized, state::PostedVaaV1};
use anchor_lang::prelude::*;

#[derive(Accounts)]
pub struct ClosePostedVaaV1<'info> {
    #[account(mut)]
    sol_destination: Signer<'info>,

    /// Posted VAA.
    #[account(
        mut,
        close = sol_destination,
        seeds = [
            PostedVaaV1::SEED_PREFIX,
            posted_vaa.message_hash().as_ref()
        ],
        bump,
    )]
    posted_vaa: Account<'info, LegacyAnchorized<4, PostedVaaV1>>,

    #[account(mut)]
    /// Signature set that may have been used to create the posted VAA account. If the `post_vaa_v1`
    /// instruction were used to create the posted VAA account, then the encoded signature set
    /// pubkey would be all zeroes.
    ///
    /// CHECK: This pubkey is checked only if this account is passed as `Some(signature_set)`. See
    /// how this is handled in the instruction handler.
    signature_set: Option<AccountInfo<'info>>,
}

pub fn close_posted_vaa_v1(ctx: Context<ClosePostedVaaV1>) -> Result<()> {
    let verified_signature_set = ctx.accounts.posted_vaa.signature_set;
    match &ctx.accounts.signature_set {
        Some(signature_set) => {
            // Verify that the signature set pubkey in the posted VAA account equals the signature
            // set pubkey passed into the account context.
            require_keys_eq!(
                signature_set.key(),
                verified_signature_set,
                CoreBridgeError::InvalidSignatureSet
            );

            // Because the signature set account may not have this implementation's discriminator,
            // we cannot load this account using Anchor's `Account` struct to then use the `close`
            // account macro directive. We have to close it using the utility method.
            crate::utils::close_account(signature_set, ctx.accounts.sol_destination.as_ref())
        }
        None => {
            // If there were no signature set used when this posted VAA was created (using the
            // `post_vaa_v1` instruction), verify that there is actually no signature set pubkey
            // written to this account.
            require_keys_eq!(
                verified_signature_set,
                Pubkey::default(),
                ErrorCode::AccountNotEnoughKeys
            );

            // Done.
            Ok(())
        }
    }
}
