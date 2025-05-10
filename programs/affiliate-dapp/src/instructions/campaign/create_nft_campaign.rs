use anchor_lang::{prelude::*, solana_program::program_option::COption};
use anchor_spl::{
    associated_token::AssociatedToken, token_2022::ID, token_interface::{transfer_checked, Mint, TokenAccount, TokenInterface, TransferChecked}
};

use crate::state::*;

#[derive(Accounts)]
#[instruction(name: String, mint_price: u64, commission_percentage: u8, campaign_details: String)]
pub struct CreateNFTCampaign<'info> {

    #[account(mut)]
    pub company: Signer<'info>,
    #[account(
        init,
        payer = company,
        space = 8 + 32 + 32 + 32 + (4 + name.len()) + 8 + 1 + (4 + campaign_details.len()) + 1 + 8 + 8,
        seeds = [b"nft_campaign", name.as_bytes()],
        bump,
        constraint = name.len() <= 32,
        constraint = campaign_details.len() <= 200,
    )]
    pub campaign: Box<Account<'info, NFTCampaign>>,
    
    #[account(
        mut,
        constraint = nft_mint.mint_authority == COption::Some(company.key())
    )]
    pub nft_mint: InterfaceAccount<'info, Mint>,
    
    

    #[account(
        mut,
        associated_token::mint = nft_mint,
        associated_token::authority = company,
        associated_token::token_program = token_program,
    )]
    pub project_token_account: InterfaceAccount<'info, TokenAccount>,

     ///CHECK: The PDA for the NFT escrow account
    #[account(
        mut,
        seeds = [b"nft_escrow",campaign.key().as_ref()],
        bump,
        
    )]
    pub nft_escrow: UncheckedAccount<'info>,
   
    #[account(
        init_if_needed,
        payer = company,
        associated_token::mint = nft_mint,
        associated_token::authority = nft_escrow, // Authority is the PDA!
    )]
    pub escrow_pda_nft_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(address = ID)]
    pub token_program: Interface<'info, TokenInterface>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}
// // Event emitted when an NFT is listed
#[event]
pub struct ListingCreatedEvent {
    pub listing: Pubkey,
    pub seller: Pubkey,
    pub nft_mint: Pubkey,
    pub price: u64,
    pub created_at: i64,
}

// // Event emitted when an NFT is sold
// #[event]
// pub struct SaleExecutedEvent {
//     pub listing: Pubkey,
//     pub seller: Pubkey,
//     pub buyer: Pubkey,
//     pub nft_mint: Pubkey,
//     pub price: u64,
//     pub sold_at: i64,
// }

// // Event emitted when a listing is canceled
// #[event]
// pub struct ListingCanceledEvent {
//     pub listing: Pubkey,
//     pub seller: Pubkey,
//     pub nft_mint: Pubkey,
//     pub canceled_at: i64,
// }

pub fn create_nft_campaign_instruction(
    ctx: Context<CreateNFTCampaign>,
    name: String,
    mint_price: u64,
    commission_percentage: u8,
    campaign_details: String,
) -> Result<()> {
    let campaign = &mut ctx.accounts.campaign;
    campaign.nft_mint =ctx.accounts.nft_mint.key(); 
    campaign.company = ctx.accounts.company.key();
    campaign.name = name;
    campaign.mint_price = mint_price;
    campaign.commission_percentage = commission_percentage;
    campaign.campaign_details = campaign_details;
    campaign.active = true; 
    campaign.affiliates_count = 0;
    campaign.total_mints = 0;
    campaign.created_at = Clock::get()?.unix_timestamp;

    emit!(ListingCreatedEvent {
        listing: campaign.key(),
        seller: campaign.company.key(),
        nft_mint:campaign.nft_mint.key(),
        price: mint_price,
        created_at: campaign.created_at,
    });
    //transfer the NFT to the escrow account

    let transfer_accounts = TransferChecked {
        from: ctx.accounts.project_token_account.to_account_info(),
        mint: ctx.accounts.nft_mint.to_account_info(),
        to: ctx.accounts.escrow_pda_nft_token_account.to_account_info(),
        authority: ctx.accounts.company.to_account_info()
    };
    // let cpi_program = ctx.accounts.token_program.to_account_info();
    // let cpi_ctx = CpiContext::new(cpi_program, transfer_instruction );
    let cpi_ctx = CpiContext::new(ctx.accounts.token_program.to_account_info(), transfer_accounts);
    transfer_checked(cpi_ctx,1,ctx.accounts.nft_mint.decimals)?;
   
    Ok(())
}
