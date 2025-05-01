use {
    anchor_lang::{
        prelude::*,
        solana_program::{
            program::invoke,
            program::invoke_signed,
            system_instruction,
        },
    },
    anchor_spl::{
        token,
        associated_token,
        metadata::{
            Metadata,
            MetadataAccount,
        },
    },
    mpl_token_metadata::{
        ID as METADATA_PROGRAM_ID,
    },
};

declare_id!("NFTMkt111111111111111111111111111111111111");

#[program]
pub mod nft_marketplace {
    use super::*;

    // Function to list an existing NFT for sale
    pub fn list_nft(
        ctx: Context<ListNft>,
        price: u64,
    ) -> Result<()> {
        // Get the current timestamp
        let clock = Clock::get()?;
        let current_time = clock.unix_timestamp;
        
        // Initialize the listing account
        let listing = &mut ctx.accounts.listing;
        listing.seller = ctx.accounts.seller.key();
        listing.nft_mint = ctx.accounts.nft_mint.key();
        listing.price = price;
        listing.is_active = true;
        listing.created_at = current_time;
        
        // Create escrow delegation using token program's approve instruction
        // This allows the marketplace to transfer the NFT when sold
        token::approve(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Approve {
                    to: ctx.accounts.seller_token_account.to_account_info(),
                    delegate: ctx.accounts.marketplace_authority.to_account_info(),
                    authority: ctx.accounts.seller.to_account_info(),
                },
            ),
            1, // Amount (always 1 for NFTs)
        )?;
        
        msg!("NFT Listed Successfully");
        msg!("Seller: {}", listing.seller);
        msg!("NFT Mint: {}", listing.nft_mint);
        msg!("Price: {}", listing.price);
        
        // Emit an event for indexers to track
        emit!(ListingCreatedEvent {
            listing: listing.key(),
            seller: ctx.accounts.seller.key(),
            nft_mint: ctx.accounts.nft_mint.key(),
            price: price,
            created_at: current_time,
        });

        Ok(())
    }
    
    // Function to buy (mint) a listed NFT
    pub fn buy_nft(ctx: Context<BuyNft>) -> Result<()> {
        let listing = &ctx.accounts.listing;
        
        // Verify listing is active
        require!(listing.is_active, NftMarketplaceError::ListingNotActive);
        
        // Get the seeds for signing as the marketplace authority
        let authority_seeds = &[
            b"marketplace_authority",
            &[ctx.bumps.marketplace_authority],
        ];
        
        // Calculate fees and royalties
        let metadata = &ctx.accounts.metadata;
        let marketplace_fee = (listing.price * 250) / 10000; // 2.5% fee
        
        // Extract royalty information from metadata
        let creators_fee_basis_points = 
            metadata.data.seller_fee_basis_points as u64;
        let creators_fee = (listing.price * creators_fee_basis_points) / 10000;
        
        // Calculate final amount for seller after fees
        let seller_amount = listing.price
            .checked_sub(marketplace_fee)
            .unwrap()
            .checked_sub(creators_fee)
            .unwrap();

        // 1. Transfer payment from buyer to seller
        invoke(
            &system_instruction::transfer(
                &ctx.accounts.buyer.key(),
                &ctx.accounts.seller.key(),
                seller_amount,
            ),
            &[
                ctx.accounts.buyer.to_account_info(),
                ctx.accounts.seller.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
        
        // 2. Transfer marketplace fee
        invoke(
            &system_instruction::transfer(
                &ctx.accounts.buyer.key(),
                &ctx.accounts.marketplace_treasury.key(),
                marketplace_fee,
            ),
            &[
                ctx.accounts.buyer.to_account_info(),
                ctx.accounts.marketplace_treasury.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
        )?;
        
        // 3. Transfer creator royalties
        // In a real implementation, you would parse each creator from metadata
        // and pay them according to their share
        // This is simplified to pay to a single creator
        if creators_fee > 0 {
            invoke(
                &system_instruction::transfer(
                    &ctx.accounts.buyer.key(),
                    &ctx.accounts.creator.key(),
                    creators_fee,
                ),
                &[
                    ctx.accounts.buyer.to_account_info(),
                    ctx.accounts.creator.to_account_info(),
                    ctx.accounts.system_program.to_account_info(),
                ],
            )?;
        }
        
        // 4. Transfer the NFT from seller to buyer using the delegation
        token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::Transfer {
                    from: ctx.accounts.seller_token_account.to_account_info(),
                    to: ctx.accounts.buyer_token_account.to_account_info(),
                    authority: ctx.accounts.marketplace_authority.to_account_info(),
                },
                &[authority_seeds],
            ),
            1, // Amount (always 1 for NFTs)
        )?;
        
        // 5. Update the listing to inactive
        let listing = &mut ctx.accounts.listing;
        listing.is_active = false;
        listing.sold_at = Some(Clock::get()?.unix_timestamp);
        listing.buyer = Some(ctx.accounts.buyer.key());
        
        msg!("NFT Purchased Successfully!");
        msg!("From Seller: {}", listing.seller);
        msg!("To Buyer: {}", ctx.accounts.buyer.key());
        msg!("NFT Mint: {}", listing.nft_mint);
        msg!("Price Paid: {}", listing.price);
        
        // Emit sale event for indexers
        emit!(SaleExecutedEvent {
            listing: listing.key(),
            seller: listing.seller,
            buyer: ctx.accounts.buyer.key(),
            nft_mint: listing.nft_mint,
            price: listing.price,
            sold_at: listing.sold_at.unwrap(),
        });

        Ok(())
    }
    
    // Function to cancel a listing
    pub fn cancel_listing(ctx: Context<CancelListing>) -> Result<()> {
        let listing = &mut ctx.accounts.listing;
        
        // Verify owner is seller
        require!(
            listing.seller == ctx.accounts.seller.key(),
            NftMarketplaceError::NotListingOwner
        );
        
        // Verify listing is still active
        require!(listing.is_active, NftMarketplaceError::ListingNotActive);
        
        // Revoke the delegation from the marketplace
        token::revoke(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                token::Revoke {
                    source: ctx.accounts.seller_token_account.to_account_info(),
                    authority: ctx.accounts.seller.to_account_info(),
                },
            ),
        )?;
        
        // Update listing to inactive
        listing.is_active = false;
        listing.canceled_at = Some(Clock::get()?.unix_timestamp);
        
        msg!("Listing Canceled Successfully");
        msg!("Seller: {}", listing.seller);
        msg!("NFT Mint: {}", listing.nft_mint);
        
        // Emit event for indexers
        emit!(ListingCanceledEvent {
            listing: listing.key(),
            seller: listing.seller,
            nft_mint: listing.nft_mint,
            canceled_at: listing.canceled_at.unwrap(),
        });

        Ok(())
    }
}

// Account structure for storing NFT listings
#[account]
pub struct Listing {
    pub seller: Pubkey,
    pub nft_mint: Pubkey,
    pub price: u64,
    pub is_active: bool,
    pub created_at: i64,
    pub sold_at: Option<i64>,
    pub canceled_at: Option<i64>,
    pub buyer: Option<Pubkey>,
}

// Context for listing an NFT
#[derive(Accounts)]
pub struct ListNft<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    
    #[account(
        init,
        payer = seller,
        space = 8 + std::mem::size_of::<Listing>(), // 8 for discriminator
        seeds = [
            b"listing",
            seller.key().as_ref(),
            nft_mint.key().as_ref(),
        ],
        bump
    )]
    pub listing: Account<'info, Listing>,
    
    pub nft_mint: Account<'info, token::Mint>,
    
    #[account(
        mut,
        associated_token::mint = nft_mint,
        associated_token::authority = seller,
    )]
    pub seller_token_account: Account<'info, token::TokenAccount>,
    
    /// CHECK: This is our marketplace authority PDA
    #[account(
        seeds = [b"marketplace_authority"],
        bump,
    )]
    pub marketplace_authority: UncheckedAccount<'info>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, token::Token>,
    pub associated_token_program: Program<'info, associated_token::AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

// Context for buying a listed NFT
#[derive(Accounts)]
pub struct BuyNft<'info> {
    #[account(mut)]
    pub buyer: Signer<'info>,
    
    #[account(
        mut,
        seeds = [
            b"listing",
            listing.seller.as_ref(),
            listing.nft_mint.as_ref(),
        ],
        bump,
        constraint = listing.is_active @ NftMarketplaceError::ListingNotActive,
    )]
    pub listing: Account<'info, Listing>,
    
    #[account(
        mut,
        address = listing.seller @ NftMarketplaceError::InvalidSellerAccount
    )]
    /// CHECK: We verify this matches the seller in the listing
    pub seller: UncheckedAccount<'info>,
    
    #[account(
        mut,
        address = listing.nft_mint @ NftMarketplaceError::InvalidNftMint
    )]
    pub nft_mint: Account<'info, token::Mint>,
    
    #[account(
        mut,
        associated_token::mint = nft_mint,
        associated_token::authority = seller,
    )]
    pub seller_token_account: Account<'info, token::TokenAccount>,
    
    #[account(
        init_if_needed,
        payer = buyer,
        associated_token::mint = nft_mint,
        associated_token::authority = buyer,
    )]
    pub buyer_token_account: Account<'info, token::TokenAccount>,
    
    /// CHECK: This is our marketplace authority PDA
    #[account(
        seeds = [b"marketplace_authority"],
        bump,
    )]
    pub marketplace_authority: UncheckedAccount<'info>,
    
    /// CHECK: This is the marketplace treasury account
    #[account(mut)]
    pub marketplace_treasury: UncheckedAccount<'info>,
    
    /// CHECK: Creator account to receive royalties
    #[account(mut)]
    pub creator: UncheckedAccount<'info>,
    
    #[account(
        seeds = [
            b"metadata",
            token_metadata_program.key().as_ref(),
            nft_mint.key().as_ref(),
        ],
        seeds::program = token_metadata_program.key(),
        bump,
    )]
    pub metadata: Account<'info, MetadataAccount>,
    
    /// CHECK: Metaplex Token Metadata Program
    #[account(address = METADATA_PROGRAM_ID)]
    pub token_metadata_program: UncheckedAccount<'info>,
    
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, token::Token>,
    pub associated_token_program: Program<'info, associated_token::AssociatedToken>,
    pub rent: Sysvar<'info, Rent>,
}

// Context for cancelling a listing
#[derive(Accounts)]
pub struct CancelListing<'info> {
    #[account(mut)]
    pub seller: Signer<'info>,
    
    #[account(
        mut,
        seeds = [
            b"listing",
            seller.key().as_ref(),
            nft_mint.key().as_ref(),
        ],
        bump,
        constraint = listing.seller == seller.key() @ NftMarketplaceError::NotListingOwner,
    )]
    pub listing: Account<'info, Listing>,
    
    pub nft_mint: Account<'info, token::Mint>,
    
    #[account(
        mut,
        associated_token::mint = nft_mint,
        associated_token::authority = seller,
    )]
    pub seller_token_account: Account<'info, token::TokenAccount>,
    
    pub token_program: Program<'info, token::Token>,
}

// Event emitted when an NFT is listed
#[event]
pub struct ListingCreatedEvent {
    pub listing: Pubkey,
    pub seller: Pubkey,
    pub nft_mint: Pubkey,
    pub price: u64,
    pub created_at: i64,
}

// Event emitted when an NFT is sold
#[event]
pub struct SaleExecutedEvent {
    pub listing: Pubkey,
    pub seller: Pubkey,
    pub buyer: Pubkey,
    pub nft_mint: Pubkey,
    pub price: u64,
    pub sold_at: i64,
}

// Event emitted when a listing is canceled
#[event]
pub struct ListingCanceledEvent {
    pub listing: Pubkey,
    pub seller: Pubkey,
    pub nft_mint: Pubkey,
    pub canceled_at: i64,
}

// Error codes for the NFT marketplace
#[error_code]
pub enum NftMarketplaceError {
    #[msg("The NFT listing is not active")]
    ListingNotActive,
    #[msg("You are not the owner of this listing")]
    NotListingOwner,
    #[msg("Invalid seller account")]
    InvalidSellerAccount,
    #[msg("Invalid NFT mint account")]
    InvalidNftMint,
}