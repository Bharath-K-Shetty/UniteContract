use anchor_lang::prelude::*;
use anchor_lang::system_program;
use solana_program::system_instruction;
declare_id!("BSCyxZZC45jfhd7dTTGgqHi2zugA36ydofso7FNdeK8X");
#[program]
pub mod unite {

    use super::*;
    const MIN_COLLATERAL: u64 = 1_000_000_000;
    pub fn initialize_organizer(ctx: Context<InitializeOrganizer>) -> Result<()> {
        let organizer = &mut ctx.accounts.organizer;
        organizer.authority = ctx.accounts.authority.key();
        organizer.event_count = 0;
        organizer.is_verified = false;
        organizer.collateral_amount = 0;
        Ok(())
    }

    pub fn create_event(
        ctx: Context<CreateEvent>,
        title: String,
        description: String,
        deadline: i64,
        ticket_price: u64,
        quorum: u32,
        maximum_capacity: u32,
    ) -> Result<()> {
        let organizer = &mut ctx.accounts.organizer;

        // Derive event_count from the organizer account
        let _event_count = organizer.event_count;

        let event = &mut ctx.accounts.event;
        event.organizer = ctx.accounts.authority.key();
        event.title = title;
        event.description = description;
        event.deadline = deadline;
        event.ticket_price = ticket_price;
        event.quorum = quorum;
        event.attendees = 0;
        event.maximum_capacity = maximum_capacity;
        event.is_cancelled = false;
        event.is_confirmed = false;
        event.bump = ctx.bumps.event;

        // Increment after using
        organizer.event_count += 1;

        Ok(())
    }
    pub fn verify_organizer(ctx: Context<VerifyOrganizer>, amount: u64) -> Result<()> {
        let organizer = &mut ctx.accounts.organizer;
        require!(!organizer.is_verified, CustomError::AlreadyVerified);
        require!(
            amount >= MIN_COLLATERAL,
            CustomError::InsufficientCollateral
        );

        // Set verified = true and store the amount
        organizer.is_verified = true;
        organizer.collateral_amount = amount;

        // Transfer SOL to collateral vault using Anchor CPI
        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.authority.to_account_info(),
                to: ctx.accounts.collateral_vault.to_account_info(),
            },
        );
        system_program::transfer(cpi_ctx, amount)?;

        Ok(())
    }

    pub fn unverify_organizer(ctx: Context<UnverifyOrganizer>) -> Result<()> {
        let organizer = &mut ctx.accounts.organizer;

        // Only allow if currently verified and has some collateral
        require!(organizer.is_verified, CustomError::NotVerified);
        require!(organizer.collateral_amount > 0, CustomError::NoCollateral);

        // Transfer SOL back to authority
        let amount = organizer.collateral_amount;
        let ix = system_instruction::transfer(
            &ctx.accounts.collateral_vault.key(),
            &ctx.accounts.authority.key(),
            amount,
        );

        // PDA signer seeds
        let seeds = &[
            b"collateral_vault",
            ctx.accounts.authority.key.as_ref(),
            &[ctx.bumps.collateral_vault],
        ];

        anchor_lang::solana_program::program::invoke_signed(
            &ix,
            &[
                ctx.accounts.collateral_vault.to_account_info(),
                ctx.accounts.authority.to_account_info(),
                ctx.accounts.system_program.to_account_info(),
            ],
            &[seeds],
        )?;

        // Reset state
        organizer.is_verified = false;
        organizer.collateral_amount = 0;

        Ok(())
    }

    pub fn buy_ticket(ctx: Context<BuyTicket>, timestamp: i64) -> Result<()> {
        let event_key = ctx.accounts.event.key();
        let event = &mut ctx.accounts.event;

        // Checks
        require!(!event.is_cancelled, CustomError::EventCancelled);
        require!(!event.is_confirmed, CustomError::EventAlreadyConfirmed);
        require!(
            event.attendees < event.maximum_capacity,
            CustomError::EventFull
        );
        require!(
            Clock::get()?.unix_timestamp < event.deadline,
            CustomError::EventClosed
        );

        // Transfer SOL using Anchor CPI
        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.buyer.to_account_info(),
                to: ctx.accounts.event_vault.to_account_info(),
            },
        );
        system_program::transfer(cpi_ctx, event.ticket_price)?;

        // Update ticket info
        let ticket = &mut ctx.accounts.ticket;
        ticket.buyer = ctx.accounts.buyer.key();
        ticket.event = event_key;
        ticket.timestamp = timestamp;
        ticket.is_refunded = false;

        // Update attendee count
        event.attendees += 1;

        Ok(())
    }
}

// ---------------------------- CONTEXTS ----------------------------

#[derive(Accounts)]
pub struct InitializeOrganizer<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + OrganizerAccount::MAX_SIZE,
        seeds = [b"organizer", authority.key().as_ref()],
        bump
    )]
    pub organizer: Account<'info, OrganizerAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct CreateEvent<'info> {
    #[account(
        mut,
        seeds = [b"organizer", authority.key().as_ref()],
        bump
    )]
    pub organizer: Account<'info, OrganizerAccount>,

    #[account(
        init,
        payer = authority,
        space = 8 + EventAccount::MAX_SIZE,
        seeds = [b"event", authority.key().as_ref(), &organizer.event_count.to_le_bytes()],
        bump
    )]
    pub event: Account<'info, EventAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct VerifyOrganizer<'info> {
    #[account(
        mut,
        seeds = [b"organizer", authority.key().as_ref()],
        bump
    )]
    pub organizer: Account<'info, OrganizerAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,
    /// CHECK: Only used to hold SOL
    #[account(
         init_if_needed,
    payer = authority,
        space = 8,
    seeds = [b"collateral_vault", authority.key().as_ref()],
    bump
)]
    pub collateral_vault: Account<'info, CollateralVaultAccount>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct UnverifyOrganizer<'info> {
    #[account(
        mut,
        seeds = [b"organizer", authority.key().as_ref()],
        bump
    )]
    pub organizer: Account<'info, OrganizerAccount>,

    #[account(mut)]
    pub authority: Signer<'info>,

    /// CHECK: Only used to hold SOL
    #[account(
    mut,
    seeds = [b"collateral_vault", authority.key().as_ref()],
    bump
)]
    pub collateral_vault: Account<'info, CollateralVaultAccount>,

    pub system_program: Program<'info, System>,
}
#[derive(Accounts)]
#[instruction(timestamp: i64)]
pub struct BuyTicket<'info> {
    #[account(mut)]
    pub event: Account<'info, EventAccount>,

    #[account(mut)]
    pub buyer: Signer<'info>,

    #[account(
        init_if_needed,
        payer = buyer,
        space = 8 + TicketAccount::MAX_SIZE,
        seeds = [b"ticket", event.key().as_ref(), buyer.key().as_ref(), &timestamp.to_le_bytes()],
        bump
    )]
    pub ticket: Account<'info, TicketAccount>,

    /// CHECK: This PDA holds SOL only, no data
    #[account(
   init_if_needed,
    payer = buyer,
        space = 8,
    seeds = [b"event_vault", event.key().as_ref()],
    bump
)]
    pub event_vault: Account<'info, EventVaultAccount>,

    pub system_program: Program<'info, System>,
}

// ---------------------------- ACCOUNTS ----------------------------

#[account]
pub struct OrganizerAccount {
    pub authority: Pubkey,
    pub event_count: u32,
    pub is_verified: bool,
    pub collateral_amount: u64,
}

impl OrganizerAccount {
    pub const MAX_SIZE: usize = 32 + 4 + 1 + 8; // Pubkey + u32
}

#[account]
pub struct EventAccount {
    pub organizer: Pubkey,
    pub title: String,
    pub description: String,
    pub deadline: i64,
    pub ticket_price: u64,
    pub quorum: u32,
    pub attendees: u32,
    pub maximum_capacity: u32,
    pub is_cancelled: bool,
    pub is_confirmed: bool,
    pub bump: u8,
}
#[account]
pub struct TicketAccount {
    pub buyer: Pubkey,
    pub event: Pubkey,
    pub timestamp: i64,
    pub is_refunded: bool,
}

#[account]
pub struct EventVaultAccount {}

#[account]
pub struct CollateralVaultAccount {}

impl TicketAccount {
    pub const MAX_SIZE: usize = 32 + 32 + 8 + 1; // buyer + event + timestamp + refunded
}

impl EventAccount {
    pub const MAX_SIZE: usize = 32 + 104 + 284 + 8 + 8 + 4 + 4 + 4 + 1 + 1 + 1;
}
#[error_code]
pub enum CustomError {
    #[msg("Organizer is not verified.")]
    NotVerified,
    #[msg("No collateral to refund.")]
    NoCollateral,
    #[msg("Event is cancelled.")]
    EventCancelled,
    #[msg("Event already confirmed.")]
    EventAlreadyConfirmed,
    #[msg("Event has reached max capacity.")]
    EventFull,
    #[msg("Event deadline has passed.")]
    EventClosed,

    #[msg("Organiser is already initialized")]
    AlreadyVerified,

    #[msg("Collateral is below the minimum threshold.")]
    InsufficientCollateral,
}
