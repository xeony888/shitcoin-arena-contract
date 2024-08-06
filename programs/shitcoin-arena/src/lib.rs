use anchor_lang::prelude::*;

declare_id!("hraFfyLS8szUAbg9xJffqCZHdgvvJQXdicjtZX9TrN3");

#[program]
pub mod shitcoin_arena {
    use super::*;

    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        msg!("Greetings from: {:?}", ctx.program_id);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct Initialize {}
