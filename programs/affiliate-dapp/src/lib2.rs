use anchor_lang::prelude::*;
use anchor_lang::system_program; // Import for SOL transfer
use anchor_spl::{
    associated_token::AssociatedToken,
    token::{self, Mint, Token, TokenAccount, Transfer as SplTransfer},
};

declare_id!("YOUR_PROGRAM_ID_HERE"); // Replace with your program ID

#[program]
pub mod nft_affiliate {
    use super::*;

    // Instruction to create a new campaign configuration
    pub fn initialize_campaign(
        ctx: Context<InitializeCampaign>,
        campaign_name: String, // Used as part of PDA seed for uniqueness
        sale_price_lamports: u64,
        commission_rate_basis_points: u16, // e.g., 500 = 5%
    ) -> Result<()> {
        require!(commission_rate_basis_points <= 10000, AffiliateError::InvalidCommissionRate);
        require!(sale_price_lamports > 0, AffiliateError::InvalidPrice);

        let campaign = &mut ctx.accounts.campaign;
        campaign.project_owner = ctx.accounts.project_owner.key();
        campaign.payout_wallet = ctx.accounts.project_payout_wallet.key(); // Use dedicated payout wallet
        campaign.sale_price_lamports = sale_price_lamports;
        campaign.commission_rate_basis_points = commission_rate_basis_points;
        campaign.active = true; // Start as active by default
        campaign.name = campaign_name; // Store the name if needed
        // Store bump for the main campaign PDA itself
        campaign.bump = *ctx.bumps.get("campaign").ok_or(ProgramError::InvalidSeeds)?;
        // Store bump for the escrow authority PDA
        campaign.escrow_bump = *ctx.bumps.get("nft_escrow_pda").ok_or(ProgramError::InvalidSeeds)?;

        msg!("Campaign initialized for project: {}", campaign.project_owner);
        msg!("Payout wallet: {}", campaign.payout_wallet);
        msg!("Price (lamports): {}", campaign.sale_price_lamports);
        msg!("Commission rate (bps): {}", campaign.commission_rate_basis_points);
        Ok(())
    }

    // Instruction for the project owner to deposit an NFT into escrow
    pub fn deposit_nft(ctx: Context<DepositNft>) -> Result<()> {
        // Ensure the authority depositing is the project owner stored in the campaign
        require_keys_eq!(
            ctx.accounts.project_owner.key(),
            ctx.accounts.campaign.project_owner,
            AffiliateError::UnauthorizedProjectOwner
        );

        // Transfer the NFT from the project owner's token account to the escrow token account
        // The escrow token account's authority is the nft_escrow_pda
        let cpi_accounts = SplTransfer {
            from: ctx.accounts.project_owner_nft_token_account.to_account_info(),
            to: ctx.accounts.escrow_pda_nft_token_account.to_account_info(),
            authority: ctx.accounts.project_owner.to_account_info(),
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new(cpi_program, cpi_accounts);
        token::transfer(cpi_ctx, 1)?; // Transfer 1 NFT

        msg!(
            "NFT {} deposited into escrow for campaign {}",
            ctx.accounts.nft_mint.key(),
            ctx.accounts.campaign.key()
        );
        Ok(())
    }

    // Instruction triggered by Blink to process the sale
    pub fn process_existing_nft_sale(ctx: Context<ProcessExistingNftSale>) -> Result<()> {
        let campaign = &ctx.accounts.campaign;
        let buyer = &ctx.accounts.buyer;
        let project_payout_wallet = &ctx.accounts.project_payout_wallet;
        let influencer_payout_wallet = &ctx.accounts.influencer_payout_wallet;

        // --- Validations ---
        require!(campaign.active, AffiliateError::CampaignNotActive);
        require_keys_eq!(
            campaign.payout_wallet,
            project_payout_wallet.key(),
            AffiliateError::IncorrectPayoutWallet
        );
        // Ensure the influencer account provided matches the key used for payout
        // The Blink action should ensure the correct influencer pubkey is in the accounts list.
        require_keys_eq!(
            ctx.accounts.influencer.key(),
            influencer_payout_wallet.key(),
            AffiliateError::InfluencerMismatch
        );
        // Check if escrow ATA holds the NFT (implicitly checked by transfer)
        require!(ctx.accounts.escrow_pda_nft_token_account.amount >= 1, AffiliateError::EscrowEmpty);
        require_keys_eq!(ctx.accounts.escrow_pda_nft_token_account.mint, ctx.accounts.nft_mint.key(), AffiliateError::MintMismatch);


        // --- Payment Distribution ---
        let price = campaign.sale_price_lamports;
        let commission_rate_bps = campaign.commission_rate_basis_points;

        let commission = price
            .checked_mul(commission_rate_bps as u64)
            .ok_or(AffiliateError::CalculationError)?
            .checked_div(10000)
            .ok_or(AffiliateError::CalculationError)?;

        let project_share = price
            .checked_sub(commission)
            .ok_or(AffiliateError::CalculationError)?;

        // Transfer SOL from buyer to project
        let ix_project = system_program::transfer(
            buyer.to_account_info().key,
            project_payout_wallet.key,
            project_share,
        );
        anchor_lang::solana_program::program::invoke(
            &ix_project,
            &[
                buyer.to_account_info(),
                project_payout_wallet.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
        msg!("Transferred {} lamports to project {}", project_share, project_payout_wallet.key());


        // Transfer SOL from buyer to influencer
        let ix_influencer = system_program::transfer(
            buyer.to_account_info().key,
            influencer_payout_wallet.key,
            commission,
        );
         anchor_lang::solana_program::program::invoke(
            &ix_influencer,
             &[
                buyer.to_account_info(),
                influencer_payout_wallet.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
        msg!("Transferred {} lamports to influencer {}", commission, influencer_payout_wallet.key());


        // --- NFT Transfer ---
        // Define PDA signer seeds for the nft_escrow_pda authority
        let campaign_key = campaign.key();
        let escrow_seeds = &[
            b"escrow".as_ref(),
            campaign_key.as_ref(),
            &[campaign.escrow_bump], // Use the stored bump
        ];
        let signer_seeds = &[&escrow_seeds[..]];

        // Create the CPI accounts for the token transfer
        let cpi_accounts = SplTransfer {
            from: ctx.accounts.escrow_pda_nft_token_account.to_account_info(),
            to: ctx.accounts.buyer_nft_token_account.to_account_info(),
            authority: ctx.accounts.nft_escrow_pda.to_account_info(), // The PDA is the authority
        };
        let cpi_program = ctx.accounts.token_program.to_account_info();
        let cpi_ctx = CpiContext::new_with_signer(cpi_program, cpi_accounts, signer_seeds);

        token::transfer(cpi_ctx, 1)?; // Transfer 1 NFT

        msg!(
            "Transferred NFT {} from escrow to buyer {}",
            ctx.accounts.nft_mint.key(),
            buyer.key()
        );

        // Optional: Could update campaign state (e.g., decrement available count)
        // Optional: Could close escrow ATA if amount is 0 and refund rent, but adds complexity.

        Ok(())
    }
}

// --- State Accounts ---

#[account]
#[derive(Default)]
pub struct Campaign {
    pub project_owner: Pubkey,
    pub payout_wallet: Pubkey,
    pub sale_price_lamports: u64,
    pub commission_rate_basis_points: u16,
    pub active: bool,
    pub name: String, // Keep reasonably short for PDA seed
    pub bump: u8,         // Bump for the Campaign PDA itself
    pub escrow_bump: u8,  // Bump for the nft_escrow_pda authority
}

// --- Instruction Contexts ---

#[derive(Accounts)]
#[instruction(campaign_name: String)]
pub struct InitializeCampaign<'info> {
    #[account(
        init,
        payer = project_owner,
        // Campaign PDA derived from owner and name for uniqueness
        seeds = [b"campaign".as_ref(), project_owner.key().as_ref(), campaign_name.as_bytes()],
        bump,
        space = 8 // Discriminator
            + 32 // project_owner
            + 32 // payout_wallet
            + 8  // sale_price_lamports
            + 2  // commission_rate_basis_points
            + 1  // active
            + 4 + campaign_name.len() // name (4 bytes prefix + length)
            + 1  // bump
            + 1, // escrow_bump
        )]
    pub campaign: Account<'info, Campaign>,

    #[account(mut)]
    pub project_owner: Signer<'info>, // The wallet creating the campaign

    /// CHECK: This is the wallet where project funds will be sent. No validation needed at init other than it's a pubkey.
    #[account(mut)] // Mutable to potentially receive rent refund if campaign closed later
    pub project_payout_wallet: AccountInfo<'info>,

    /// The PDA that will act as the authority for NFT escrow token accounts for this campaign
    #[account(
        seeds = [b"escrow".as_ref(), campaign.key().as_ref()],
        bump
    )]
    pub nft_escrow_pda: SystemAccount<'info>, // Or AccountInfo<'info> if preferred

    pub system_program: Program<'info, System>,
}


#[derive(Accounts)]
pub struct DepositNft<'info> {
    #[account(
        mut, // May need mut if you update stats on it later
        has_one = project_owner @ AffiliateError::UnauthorizedProjectOwner, // Ensure signer is owner
        seeds = [b"campaign".as_ref(), campaign.project_owner.as_ref(), campaign.name.as_bytes()], // Use correct seeds
        bump = campaign.bump,
    )]
    pub campaign: Account<'info, Campaign>,

    #[account(mut)]
    pub project_owner: Signer<'info>, // Must be the signer

    // NFT details
    pub nft_mint: Account<'info, Mint>,

    // Project owner's token account holding the NFT to be deposited
    #[account(
        mut,
        associated_token::mint = nft_mint,
        associated_token::authority = project_owner,
    )]
    pub project_owner_nft_token_account: Account<'info, TokenAccount>,

    // The escrow authority PDA (derives the escrow ATA)
    /// CHECK: This PDA's seeds are validated below. It acts as the authority.
    #[account(
        seeds = [b"escrow".as_ref(), campaign.key().as_ref()],
        bump = campaign.escrow_bump // Use stored bump
    )]
    pub nft_escrow_pda: AccountInfo<'info>,

    // Escrow's Associated Token Account for this specific NFT Mint
    // Initialized by anchor/program if needed using the nft_escrow_pda as authority
    #[account(
        init_if_needed,
        payer = project_owner,
        associated_token::mint = nft_mint,
        associated_token::authority = nft_escrow_pda, // Authority is the PDA!
    )]
    pub escrow_pda_nft_token_account: Account<'info, TokenAccount>,

    // Programs
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>, // Needed for init_if_needed
}


#[derive(Accounts)]
pub struct ProcessExistingNftSale<'info> {
     #[account(
        // No mut needed if only reading state during processing
        constraint = campaign.active @ AffiliateError::CampaignNotActive, // Ensure active
        seeds = [b"campaign".as_ref(), campaign.project_owner.as_ref(), campaign.name.as_bytes()], // Use correct seeds
        bump = campaign.bump,
        has_one = project_payout_wallet @ AffiliateError::IncorrectPayoutWallet, // Ensure payout wallet matches
    )]
    pub campaign: Account<'info, Campaign>,

    #[account(mut)] // Buyer pays SOL and receives NFT
    pub buyer: Signer<'info>,

    /// CHECK: Influencer account passed in transaction. Verified against influencer_payout_wallet.
    pub influencer: AccountInfo<'info>,

    // NFT details
    #[account(mut)] // Mint account doesn't usually need mut for transfer
    pub nft_mint: Account<'info, Mint>,

    // Escrow authority PDA
    /// CHECK: Seeds verified below. Acts as authority for transfer FROM escrow ATA.
    #[account(
        seeds = [b"escrow".as_ref(), campaign.key().as_ref()],
        bump = campaign.escrow_bump
    )]
    pub nft_escrow_pda: AccountInfo<'info>,

    // Escrow's ATA holding the specific NFT being sold
    #[account(
        mut, // Needs mut to transfer *from*
        associated_token::mint = nft_mint,
        associated_token::authority = nft_escrow_pda, // PDA is authority
    )]
    pub escrow_pda_nft_token_account: Account<'info, TokenAccount>,

     // Buyer's ATA for this specific NFT Mint
    // Anchor initializes if needed
    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = nft_mint,
        associated_token::authority = buyer, // Buyer is the authority
    )]
    pub buyer_nft_token_account: Account<'info, TokenAccount>,

    // Payout Wallets
    /// CHECK: Wallet address verified against campaign.payout_wallet constraint.
    #[account(mut)]
    pub project_payout_wallet: AccountInfo<'info>,

    /// CHECK: Wallet address verified against influencer.key() in instruction logic.
    #[account(mut)]
    pub influencer_payout_wallet: AccountInfo<'info>,

    // Programs
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
    pub rent: Sysvar<'info, Rent>, // Needed for init_if_needed
}

// --- Errors ---

#[error_code]
pub enum AffiliateError {
    #[msg("The commission rate must be between 0 and 10000 basis points.")]
    InvalidCommissionRate,
    #[msg("The sale price must be greater than zero.")]
    InvalidPrice,
    #[msg("Signer is not the authorized project owner for this campaign.")]
    UnauthorizedProjectOwner,
    #[msg("The campaign is not currently active.")]
    CampaignNotActive,
    #[msg("The provided project payout wallet does not match the campaign configuration.")]
    IncorrectPayoutWallet,
    #[msg("The provided influencer payout wallet does not match the influencer account.")]
    InfluencerMismatch,
    #[msg("Calculation resulted in overflow or underflow.")]
    CalculationError,
    #[msg("The NFT escrow account for this mint is empty or does not exist.")]
    EscrowEmpty,
     #[msg("The Mint account provided does not match the escrow token account's mint.")]
    MintMismatch,
}