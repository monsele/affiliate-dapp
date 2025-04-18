use anchor_lang::{prelude::*, solana_program::instruction::Instruction};
use crate::state::*;
use anchor_lang::solana_program::program::{invoke};
#[derive(Accounts)]
pub struct ProcessAffiliateMint<'info> {
    #[account(mut)]
    pub campaign: Account<'info, NFTCampaign>,
    
    #[account(mut, has_one = campaign)]
    pub affiliate_link: Account<'info, AffiliateLink>,
    
    // The user buying the NFT
    #[account(mut)]
    pub buyer: Signer<'info>,
    
    /// CHECK: Only reading the address for payment
    #[account(mut, address = affiliate_link.influencer)]
    pub influencer: UncheckedAccount<'info>,
    /// CHECK: Only reading the address for payment
    #[account(mut, address = campaign.company)]
    pub owner: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

// Process an NFT mint through an affiliate link
pub fn process_affiliate_mint_instruction(
    ctx: Context<ProcessAffiliateMint>,
    mint_instruction_data: Vec<u8>,
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
        &campaign.nft_project,
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
    let accounts = ctx.remaining_accounts.iter().map(|acc| {
        if acc.is_writable {
            AccountMeta::new(acc.key(), acc.is_signer)
        } else {
            AccountMeta::new_readonly(acc.key(), acc.is_signer)
        }
    }).collect();
    
    let nft_mint_ix = Instruction {
        program_id: campaign.nft_mint_program,
        accounts,
        data: mint_instruction_data,
    };
    
    invoke(
        &nft_mint_ix,
        ctx.remaining_accounts,
    )?;
    
    // Update stats
    campaign.total_mints += 1;
    affiliate_link.mints_count += 1;
    affiliate_link.earnings += commission_amount as u64;
    
    Ok(())
}

