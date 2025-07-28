use anchor_lang::prelude::*;

declare_id!("HgYaGfeHEfgM6BFeTCGZsP9rNoTLkrnsxXv3nsAJQwuF");

#[program]
pub mod unite {
    use super::*;

    pub fn initialize_organizer(ctx: Context<InitializeOrganizer>) -> Result<()> {
        let organizer = &mut ctx.accounts.organizer;
        organizer.authority = ctx.accounts.authority.key();
        organizer.event_count = 0;
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

// ---------------------------- ACCOUNTS ----------------------------

#[account]
pub struct OrganizerAccount {
    pub authority: Pubkey,
    pub event_count: u32,
}

impl OrganizerAccount {
    pub const MAX_SIZE: usize = 32 + 4; // Pubkey + u32
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

impl EventAccount {
    pub const MAX_SIZE: usize = 32 + 104 + 284 + 8 + 8 + 4 + 4 + 4 + 1 + 1 + 1;
}