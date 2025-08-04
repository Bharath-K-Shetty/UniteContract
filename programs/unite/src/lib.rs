use anchor_lang::prelude::*;
use anchor_lang::system_program;

declare_id!("DvjeoH6mkwS9BUotv3azz9Wr7NcSatM2xhvYpoKaVRqZ");
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
        city: String,
        address: String,
        image_url: String,
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
        event.city = city;
        event.address = address;
        event.image_url = image_url;
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
        let rent = Rent::get()?;
        let min_balance = rent.minimum_balance(0);
        require!(amount >= min_balance, CustomError::InsufficientCollateral);

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
        let seeds: &[&[u8]] = &[
            b"collateral_vault",
            ctx.accounts.authority.key.as_ref(),
            &[ctx.bumps.collateral_vault],
        ];

        let signer_seeds: &[&[&[u8]]] = &[seeds];

        let cpi_ctx = CpiContext::new(
            ctx.accounts.system_program.to_account_info(),
            system_program::Transfer {
                from: ctx.accounts.collateral_vault.to_account_info(),
                to: ctx.accounts.authority.to_account_info(),
            },
        )
        .with_signer(signer_seeds);

        system_program::transfer(cpi_ctx, amount)?;

        // Reset state
        organizer.is_verified = false;
        organizer.collateral_amount = 0;

        Ok(())
    }

    pub fn initialize_ticket_account(
        ctx: Context<InitializeTicketAccount>,
        timestamp: i64,
    ) -> Result<()> {
        let ticket = &mut ctx.accounts.ticket;
        let event = &mut ctx.accounts.event;
        let rent = Rent::get()?; // ⬅️ Get Rent sysvar
        let min_balance = rent.minimum_balance(0); // 0 bytes, system account
        require!(
            event.ticket_price >= min_balance,
            CustomError::InsufficientCollateral
        );

        ticket.buyer = ctx.accounts.buyer.key();
        ticket.timestamp = timestamp;
        ticket.event = ctx.accounts.event.key();
        ticket.is_refunded = false;

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
        mut,
            seeds = [b"collateral_vault", authority.key().as_ref()],
            bump
        )]
    pub collateral_vault: UncheckedAccount<'info>,

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

    /// CHECK: This is a system-owned PDA used only to hold SOL
    #[account(
        mut,
            seeds = [b"collateral_vault", authority.key().as_ref()],
            bump
        )]
    pub collateral_vault: UncheckedAccount<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
#[instruction(timestamp: i64)]
pub struct InitializeTicketAccount<'info> {
    #[account(
                init,
                payer = buyer,
                space = 8 + TicketAccount::MAX_SIZE,
                seeds = [b"ticket", event.key().as_ref(), buyer.key().as_ref(), &timestamp.to_le_bytes()],
                bump
            )]
    pub ticket: Account<'info, TicketAccount>,

    #[account(mut)]
    pub buyer: Signer<'info>,

    pub event: Account<'info, EventAccount>,

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
                mut,
                seeds = [b"ticket", event.key().as_ref(), buyer.key().as_ref(), &timestamp.to_le_bytes()],
                bump
            )]
    pub ticket: Account<'info, TicketAccount>,

    /// CHECK: This is a system-owned PDA used only to hold SOL
    #[account(
            mut,
            seeds = [b"event_vault", event.key().as_ref()],
            bump
        )]
    pub event_vault: UncheckedAccount<'info>,

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
    pub const MAX_SIZE: usize = 32 + 4 + 1 + 8;
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
    pub city: String,
    pub address: String,
    pub image_url: String,
}
#[account]
pub struct TicketAccount {
    pub buyer: Pubkey,
    pub event: Pubkey,
    pub timestamp: i64,
    pub is_refunded: bool,
}

impl TicketAccount {
    pub const MAX_SIZE: usize = 32 + 32 + 8 + 1; // buyer + event + timestamp + refunded
}

impl EventAccount {
    pub const MAX_SIZE: usize = 32 + // organizer
        4 + 100 + // title: up to 100 bytes (4 bytes for prefix)
        4 + 256 + // description: up to 256 bytes
        8 + // deadline
        8 + // ticket_price
        4 + // quorum
        4 + // attendees
        4 + // maximum_capacity
        1 + // is_cancelled
        1 + // is_confirmed
        1 + // bump
        4 + 50 + // city: up to 50 bytes
        4 + 100+
        4+150;
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
