use anchor_lang::prelude::*;
use anchor_spl::token::{self, Token, TokenAccount};
use spl_token::instruction::approve;

declare_id!("YourProgramID");

#[program]
pub mod nft_approval_example {
    use super::*;

    // Approve a PDA to transfer the user's NFT
    pub fn approve_nft_delegate(ctx: Context<ApproveNftDelegate>) -> Result<()> {
        // Verify the token is an NFT (simplified check)
        let token_account = &ctx.accounts.source_token_account;
        require!(token_account.amount == 1, ErrorCode::NotAnNft);
        require!(token_account.decimals == 0, ErrorCode::NotAnNft);

        // Build the `approve` instruction for 1 token
        let approve_instruction = approve(
            &spl_token::ID,
            &token_account.key(),
            &ctx.accounts.delegate_pda.key(),
            &ctx.accounts.owner.key(),
            &[],
            1, // Approve 1 NFT
        )?;

        // Invoke the SPL Token Program via CPI
        anchor_lang::solana_program::program::invoke(
            &approve_instruction,
            &[
                token_account.to_account_info(),
                ctx.accounts.delegate_pda.to_account_info(),
                ctx.accounts.owner.to_account_info(),
                ctx.accounts.token_program.to_account_info(),
            ],
        )?;

        Ok(())
    }
}

#[derive(Accounts)]
pub struct ApproveNftDelegate<'info> {
    #[account(mut)]
    pub source_token_account: Account<'info, TokenAccount>, // User's NFT token account
    pub delegate_pda: AccountInfo<'info>,                   // dApp's PDA (delegate)
    #[account(mut)]
    pub owner: Signer<'info>,                               // NFT owner (user)
    pub token_program: Program<'info, Token>,               // SPL Token Program
}

#[error_code]
pub enum ErrorCode {
    #[msg("Token is not an NFT")]
    NotAnNft,
}