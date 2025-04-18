use anchor_lang::prelude::*;
use anchor_lang::solana_program::{
    instruction::Instruction, 
    program::invoke, 
    program::invoke_signed
};
use anchor_spl::token::{Token, TokenAccount};

declare_id!("Affi1iateMarketing111111111111111111111111111");

#[program]
pub mod nft_affiliate_marketing {
    use super::*;

    // Create a new campaign for an NFT project
    pub fn create_nft_campaign(
        ctx: Context<CreateNFTCampaign>,
        name: String,
        mint_price: u64,
        commission_percentage: u8,
        campaign_details: String,
    ) -> Result<()> {
        let campaign = &mut ctx.accounts.campaign;
        campaign.nft_project = ctx.accounts.nft_project.key();
        campaign.nft_mint_program = ctx.accounts.nft_mint_program.key();
        campaign.company = ctx.accounts.company.key();
        campaign.name = name;
        campaign.mint_price = mint_price;
        campaign.commission_percentage = commission_percentage;
        campaign.campaign_details = campaign_details;
        campaign.active = true;
        campaign.affiliates_count = 0;
        campaign.total_mints = 0;
        
        Ok(())
    }

    // Allow influencers to create affiliate links for campaigns
    pub fn create_affiliate_link(
        ctx: Context<CreateAffiliateLink>,
        campaign_id: Pubkey,
    ) -> Result<()> {
        let affiliate_link = &mut ctx.accounts.affiliate_link;
        let campaign = &mut ctx.accounts.campaign;
        
        affiliate_link.campaign = campaign.key();
        affiliate_link.influencer = ctx.accounts.influencer.key();
        affiliate_link.mints_count = 0;
        affiliate_link.earnings = 0;
        
        campaign.affiliates_count += 1;
        
        Ok(())
    }

    // Process an NFT mint through an affiliate link
    pub fn process_affiliate_mint(
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
                ctx.accounts.nft_project.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;

        // 3. Call the NFT project's mint instruction
        let accounts = ctx.remaining_accounts.to_vec();
        
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
}

// Account structures
#[derive(Accounts)]
#[instruction(name: String, mint_price: u64, commission_percentage: u8, campaign_details: String)]
pub struct CreateNFTCampaign<'info> {
    #[account(
        init,
        payer = company,
        space = 8 + 32 + 32 + 32 + (4 + name.len()) + 8 + 1 + (4 + campaign_details.len()) + 1 + 8 + 8
    )]
    pub campaign: Account<'info, NFTCampaign>,
    
    // NFT project's main wallet
    /// CHECK: This account is not being read or written
    pub nft_project: UncheckedAccount<'info>,
    
    // The program ID that handles minting for this NFT collection
    /// CHECK: This account is not being read or written
    pub nft_mint_program: UncheckedAccount<'info>,
    
    #[account(mut)]
    pub company: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(campaign_id: Pubkey)]
pub struct CreateAffiliateLink<'info> {
    #[account(
        init,
        payer = influencer,
        space = 8 + 32 + 32 + 8 + 8
    )]
    pub affiliate_link: Account<'info, AffiliateLink>,
    
    #[account(mut, address = campaign_id)]
    pub campaign: Account<'info, NFTCampaign>,
    
    #[account(mut)]
    pub influencer: Signer<'info>,
    pub system_program: Program<'info, System>,
}

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
    
    pub system_program: Program<'info, System>,
}

// State definitions
#[account]
pub struct NFTCampaign {
    pub nft_project: Pubkey,          // Wallet of the NFT project
    pub nft_mint_program: Pubkey,     // Program ID that handles minting
    pub company: Pubkey,              // Campaign creator
    pub name: String,                 // Campaign name
    pub mint_price: u64,              // Price to mint one NFT
    pub commission_percentage: u8,    // Commission percentage (0-100)
    pub campaign_details: String,     // Additional details
    pub active: bool,                 // Campaign status
    pub affiliates_count: u64,        // Number of affiliates
    pub total_mints: u64,             // Total NFTs minted via affiliates
}

#[account]
pub struct AffiliateLink {
    pub campaign: Pubkey,        // Campaign this affiliate link belongs to
    pub influencer: Pubkey,      // Wallet of the influencer
    pub mints_count: u64,        // Number of mints through this link
    pub earnings: u64,           // Total earnings in lamports
}