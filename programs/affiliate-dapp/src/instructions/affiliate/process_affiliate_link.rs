use anchor_lang::{prelude::*, solana_program::program::invoke};

use anchor_spl::{
    associated_token::AssociatedToken,
    
    token_interface::{
        spl_token_2022::ID as TOKEN_2022_PROGRAM_ID,
        Mint, TokenAccount, TokenInterface,TransferChecked, transfer_checked
    },
};
use crate::{error::CustomError, state::*};

#[derive(Accounts)]
#[instruction(campaign_name: String, influencer: Pubkey)]
pub struct ProcessAffiliateMint<'info> {
    #[account(
        mut,
        seeds = [b"nft_campaign", campaign_name.as_bytes()],
        bump,
        constraint = campaign.active == true
    )]
    pub campaign: Account<'info, NFTCampaign>,

    #[account(
        mut,
        seeds = [b"affiliate_link", influencer.key().as_ref(), campaign_name.as_bytes()],
        bump
    )]
    pub affiliate_link: Account<'info, AffiliateLink>,

    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(mut)]
    /// CHECK: Validated in constraint
    pub owner: UncheckedAccount<'info>,
     /// CHECK: Validated in constraint
    #[account(mut)]
    pub influencer: AccountInfo<'info>,
    
    #[account(
        address = campaign.nft_mint
    )]
    pub nft_mint: InterfaceAccount<'info, Mint>,
    
     /// CHECK: Validated in constraint
      #[account(
        mut,
        seeds = [b"nft_escrow",campaign.key().as_ref()],
        bump,
        
    )]
    pub nft_escrow: UncheckedAccount<'info>,
    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = nft_mint,
        associated_token::authority = buyer,
        associated_token::token_program = token_program
    )]
    pub buyer_token_account: InterfaceAccount<'info, TokenAccount>,
    
    #[account(
        mut,
        associated_token::mint = campaign.nft_mint,
        associated_token::authority = owner.key(),
        associated_token::token_program = token_program,
        owner = TOKEN_2022_PROGRAM_ID,
    )]
    pub owner_token_account: InterfaceAccount<'info, TokenAccount>,
 #[account(
        mut,
        associated_token::mint = nft_mint,
        associated_token::authority = nft_escrow, // Authority is the PDA!
        associated_token::token_program = token_program,
    )]

   pub escrow_pda_nft_token_account: InterfaceAccount<'info, TokenAccount>,
    #[account(
        seeds = [b"marketplace_authority"],
        bump,
    )]
    pub marketplace_authority: SystemAccount<'info>,
    
    #[account(address = TOKEN_2022_PROGRAM_ID)]
    pub token_program: Interface<'info, TokenInterface>,
    
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}


pub fn process_affiliate_mint_instruction(ctx: Context<ProcessAffiliateMint>, _campaign_name:String, _influencer:Pubkey) -> Result<()> {
    let campaign = &mut ctx.accounts.campaign;
    let affiliate_link = &mut ctx.accounts.affiliate_link;
    let campaign_key = campaign.key().clone();
    // Calculate commission amount with proper overflow checks
    let commission_amount = (campaign.mint_price as u64)
        .checked_mul(campaign.commission_percentage as u64)
        .ok_or(ProgramError::ArithmeticOverflow)?
        .checked_div(100)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Calculate amount going to the NFT project
    let project_amount = campaign.mint_price
        .checked_sub(commission_amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // 1. Transfer commission to the influencer
    // Verify influencer account matches the one in the affiliate link
    require!(
        ctx.accounts.influencer.key() == affiliate_link.influencer,
        CustomError::InvalidInfluencer
    );
    
    let transfer_to_influencer_ix = anchor_lang::solana_program::system_instruction::transfer(
        &ctx.accounts.buyer.key(),
        &affiliate_link.influencer,
        commission_amount,
    );

    invoke(
        &transfer_to_influencer_ix,
        &[
            ctx.accounts.buyer.to_account_info(),
            ctx.accounts.influencer.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    // 2. Transfer remaining amount to the NFT project
    // Verify owner account matches the company in the campaign
    require!(
        ctx.accounts.owner.key() == campaign.company,
        CustomError::InvalidAccountOwner
    );
    
    let transfer_to_project_ix = anchor_lang::solana_program::system_instruction::transfer(
        &ctx.accounts.buyer.key(),
        &campaign.company,
        project_amount,
    );

    invoke(
        &transfer_to_project_ix,
        &[
            ctx.accounts.buyer.to_account_info(),
            ctx.accounts.owner.to_account_info(),
            ctx.accounts.system_program.to_account_info(),
        ],
    )?;

    // Get the seeds for signing as the escrow PDA
    let escrow_seeds = [
        b"nft_escrow",
        campaign_key.as_ref(),
        &[ctx.bumps.nft_escrow],
    ];
    let escrow_seeds_ref = [&escrow_seeds[..]];

    let transfer_accounts = TransferChecked {
        from: ctx.accounts.escrow_pda_nft_token_account.to_account_info(),
        mint: ctx.accounts.nft_mint.to_account_info(),
        to: ctx.accounts.buyer_token_account.to_account_info(),
        authority: ctx.accounts.nft_escrow.to_account_info()
    };
   
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(), 
        transfer_accounts,
        &escrow_seeds_ref
    );
    
    transfer_checked(cpi_ctx, 1, ctx.accounts.nft_mint.decimals)?;

    // Update stats
    campaign.total_mints += 1;
    affiliate_link.mints_count += 1;
    affiliate_link.earnings += commission_amount;

    Ok(())
}