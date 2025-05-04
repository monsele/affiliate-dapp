use anchor_lang::{prelude::*, solana_program::program::invoke};
use anchor_lang::solana_program::program_option::COption;
use anchor_spl::token;
use anchor_spl::{
    associated_token::AssociatedToken,
    token_interface::{
        spl_token_2022::ID as TOKEN_2022_PROGRAM_ID,
        Mint, TokenAccount, TokenInterface
    },
};
use crate::state::*;

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
        address = campaign.nft_mint,
        owner = TOKEN_2022_PROGRAM_ID,
        constraint = nft_mint.mint_authority == COption::Some(marketplace_authority.key())
    )]
    pub nft_mint: InterfaceAccount<'info, Mint>,
    
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
        seeds = [b"marketplace_authority"],
        bump,
    )]
    pub marketplace_authority: SystemAccount<'info>,
    
    #[account(address = TOKEN_2022_PROGRAM_ID)]
    pub token_program: Interface<'info, TokenInterface>,
    
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

// Process an NFT mint through an affiliate link
pub fn process_affiliate_mint_instruction(
    ctx: Context<ProcessAffiliateMint>
) -> Result<()> {
    let campaign = &mut ctx.accounts.campaign;
    let affiliate_link = &mut ctx.accounts.affiliate_link;

    // Calculate commission amount
    let commission_amount = (campaign.mint_price as u128)
        .checked_mul(campaign.commission_percentage as u128)
        .unwrap()
        .checked_div(100)
        .unwrap();

    // Calculate amount going to the NFT project
    let project_amount = campaign.mint_price.checked_sub(commission_amount as u64).unwrap();

    // 1. Transfer commission to the influencer
    let transfer_to_influencer_ix = anchor_lang::solana_program::system_instruction::transfer(
        &ctx.accounts.buyer.key(),
        &affiliate_link.influencer,
        commission_amount as u64,
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

    // 3. Call the NFT project's mint instruction
    // let accounts = ctx.remaining_accounts.iter().map(|acc| {
    //     if acc.is_writable {
    //         AccountMeta::new(acc.key(), acc.is_signer)
    //     } else {
    //         AccountMeta::new_readonly(acc.key(), acc.is_signer)
    //     }
    // }).collect();

    // let nft_mint_ix = Instruction {
    //     program_id: campaign.nft_mint_program,
    //     accounts,
    //     data: mint_instruction_data,
    // };

    // invoke(
    //     &nft_mint_ix,
    //     ctx.remaining_accounts,
    // )?;
    // Get the seeds for signing as the marketplace authority
    let seeds = &[
        b"marketplace_authority".as_ref(),
        &[ctx.bumps.marketplace_authority],
    ];

//let signer_seeds = &[authority_seeds]; // `&[&[u8]][]` for invoke_signed
    token::transfer(
        CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.owner_token_account.to_account_info(),
                to: ctx.accounts.buyer_token_account.to_account_info(),
                authority: ctx.accounts.marketplace_authority.to_account_info(),
            },
            &[seeds],
        ),
        1, // Amount (always 1 for NFTs)
    )?;
    // Update stats
    campaign.total_mints += 1;
    affiliate_link.mints_count += 1;
    affiliate_link.earnings += commission_amount as u64;

    Ok(())
}

