use anchor_lang::{prelude::*, solana_program::native_token::LAMPORTS_PER_SOL};
use anchor_spl::associated_token::AssociatedToken;
use anchor_spl::token::{Mint, Token, TokenAccount};
use anchor_spl::token::{mint_to, set_authority, Transfer, spl_token::instruction::AuthorityType, transfer, MintTo, SetAuthority};

declare_id!("hraFfyLS8szUAbg9xJffqCZHdgvvJQXdicjtZX9TrN3");

const MINT_DECIMALS: u8 = 6;
const FEE_LAMPORTS: u64 = LAMPORTS_PER_SOL / 10;
const FEE_PERCENT_BP: u64 = 1;
const TOKEN_SUPPLY: u64 = 1000000000 * 10_u64.pow(6);
 // adjusts expressions so that 20% of the supply will be in circulation when the mkt cap is 420 sol
const DENOMINATOR_C: u64 = 21 * 10_u64.pow(17);
const ADMIN: &str = "";
const TARGET_MARKET_CAP: u64 = 690 * LAMPORTS_PER_SOL;
#[program]
pub mod shitcoin_arena {
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>) -> Result<()> {
        Ok(())
    }
    pub fn create_token_and_buy(ctx: Context<CreateTokenAndBuy>, amount: u64) -> Result<()> {
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.signer.to_account_info(),
                    to: ctx.accounts.program_sol_account.to_account_info()
                }
            ),
            FEE_LAMPORTS,
        )?;
        mint_to(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                MintTo {
                    to: ctx.accounts.curve_token_account.to_account_info(),
                    mint: ctx.accounts.mint.to_account_info(),
                    authority: ctx.accounts.program_authority.to_account_info()
                },
                &[&[b"auth", &[ctx.bumps.program_authority]]]
            ),
            TOKEN_SUPPLY
        )?;
        set_authority(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                SetAuthority {
                    current_authority: ctx.accounts.program_authority.to_account_info(),
                    account_or_mint: ctx.accounts.mint.to_account_info()
                },
                &[&[b"auth", &[ctx.bumps.program_authority]]]
            ),
            AuthorityType::MintTokens,
            None
        )?;
        ctx.accounts.bonding_curve.closed = false;
        if amount > 0 {
            let lamports = ctx.accounts.bonding_curve.buy(amount);
            let fee_lamports = lamports * FEE_PERCENT_BP / 100 / 100;
            anchor_lang::system_program::transfer(
                CpiContext::new(
                    ctx.accounts.system_program.to_account_info(),
                    anchor_lang::system_program::Transfer {
                        from: ctx.accounts.signer.to_account_info(),
                        to: ctx.accounts.curve_sol_account.to_account_info(),
                    }
                ),
                lamports
            )?;
            anchor_lang::system_program::transfer(
                CpiContext::new(
                    ctx.accounts.system_program.to_account_info(),
                    anchor_lang::system_program::Transfer {
                        from: ctx.accounts.signer.to_account_info(),
                        to: ctx.accounts.program_sol_account.to_account_info()
                    }
                ),
                fee_lamports
            )?;
            transfer(
                CpiContext::new_with_signer(
                    ctx.accounts.token_program.to_account_info(),
                    Transfer {
                        from: ctx.accounts.curve_token_account.to_account_info(),
                        to: ctx.accounts.signer_token_account.to_account_info(),
                        authority: ctx.accounts.program_authority.to_account_info(),
                    },
                    &[&[b"auth", &[ctx.bumps.program_authority]]]
                ),
                amount
            )?;
        }
        Ok(())
    }
    pub fn buy(ctx: Context<Buy>, amount: u64) -> Result<()> {
        if ctx.accounts.bonding_curve.closed {
            return Err(CustomError::CurveInMigration.into())
        }
        let lamports = ctx.accounts.bonding_curve.buy(amount);
        let fee_lamports = lamports * FEE_PERCENT_BP / 100 / 100;

        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.signer.to_account_info(),
                    to: ctx.accounts.curve_sol_account.to_account_info(),
                }
            ),
            lamports
        )?;
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.signer.to_account_info(),
                    to: ctx.accounts.program_sol_account.to_account_info()
                }
            ),
            fee_lamports
        )?;
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.curve_token_account.to_account_info(),
                    to: ctx.accounts.signer_token_account.to_account_info(),
                    authority: ctx.accounts.program_authority.to_account_info(),
                },
                &[&[b"auth", &[ctx.bumps.program_authority]]]
            ),
            amount
        )?;
        if ctx.accounts.bonding_curve.mkt_cap() >= TARGET_MARKET_CAP {
            ctx.accounts.bonding_curve.closed = true;
            emit!(InitializeMigrateEvent {
                mint: ctx.accounts.mint.key()
            });
        }
        Ok(())
    }   
    pub fn sell(ctx: Context<Sell>, amount: u64) -> Result<()> {
        if ctx.accounts.bonding_curve.closed {
            return Err(CustomError::CurveInMigration.into())
        }
        let lamports = ctx.accounts.bonding_curve.sell(amount);
        let fee_lamports = lamports * FEE_PERCENT_BP / 100 / 100;
        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.signer_token_account.to_account_info(),
                    to: ctx.accounts.curve_token_account.to_account_info(),
                    authority: ctx.accounts.signer.to_account_info()
                }
            ),
            amount,
        )?;
        **ctx.accounts.program_sol_account.try_borrow_mut_lamports()? -= lamports;
        **ctx.accounts.signer.try_borrow_mut_lamports()? += lamports;
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.signer.to_account_info(),
                    to: ctx.accounts.program_sol_account.to_account_info(),
                }
            ),
            fee_lamports
        )?;
        Ok(())
    }
    pub fn swap(ctx: Context<Swap>, sell_amount: u64, buy_amount: u64) -> Result<()> {
        let sell_lamports = ctx.accounts.from_curve.sell(sell_amount);
        let sell_fee = sell_lamports * FEE_PERCENT_BP / 100 / 100;
        let buy_lamports = ctx.accounts.to_curve.buy(buy_amount);
        let buy_fee = buy_lamports * FEE_PERCENT_BP / 100 / 100; 
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.signer.to_account_info(),
                    to: ctx.accounts.program_sol_account.to_account_info()
                }
            ),
            sell_fee + buy_fee,
        )?;
        **ctx.accounts.from_curve_sol_account.try_borrow_mut_lamports()? -= sell_amount;
        **ctx.accounts.signer.try_borrow_mut_lamports()? += sell_amount;
        anchor_lang::system_program::transfer(
            CpiContext::new(
                ctx.accounts.system_program.to_account_info(),
                anchor_lang::system_program::Transfer {
                    from: ctx.accounts.signer.to_account_info(),
                    to: ctx.accounts.to_curve_sol_account.to_account_info()
                }
            ),
            buy_lamports,
        )?;
        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.signer_from_token_account.to_account_info(),
                    to: ctx.accounts.from_curve_token_account.to_account_info(),
                    authority: ctx.accounts.signer.to_account_info()
                }
            ),
            sell_amount
        )?;
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.to_curve_token_account.to_account_info(),
                    to: ctx.accounts.signer_to_token_account.to_account_info(),
                    authority: ctx.accounts.program_authority.to_account_info()
                },
                &[&[b"auth", &[ctx.bumps.program_authority]]]
            ),
            sell_amount
        )?;
        Ok(())
    }
    pub fn withdraw_fees(ctx: Context<WithdrawFees>) -> Result<()> {
        if ADMIN.parse::<Pubkey>().unwrap() != ctx.accounts.signer.key() {
            return Err(CustomError::InvalidSigner.into())
        }
        let min_rent = Rent::get()?.minimum_balance(8) + 1;
        let lamports = ctx.accounts.program_sol_account.get_lamports() - min_rent;
        if lamports <= 0 {
            return Err(CustomError::NoFeesToWithdraw.into())
        }
        **ctx.accounts.program_sol_account.try_borrow_mut_lamports()? -= lamports;
        **ctx.accounts.signer.try_borrow_mut_lamports()? += lamports;
        Ok(())
    }
    pub fn migrate(ctx: Context<Migrate>) -> Result<()> {
        if ADMIN.parse::<Pubkey>().unwrap() != ctx.accounts.signer.key() {
            return Err(CustomError::InvalidSigner.into())
        }
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.curve_token_account.to_account_info(),
                    to: ctx.accounts.token_recieving_account.to_account_info(),
                    authority: ctx.accounts.program_authority.to_account_info()
                },
                &[&[b"auth", &[ctx.bumps.program_authority]]]
            ),
            ctx.accounts.curve_token_account.amount
        )?;
        let min_rent = Rent::get()?.minimum_balance(8) + 1;
        let lamports = ctx.accounts.curve_sol_account.get_lamports() - min_rent;
        **ctx.accounts.curve_sol_account.try_borrow_mut_lamports()? -= lamports;
        **ctx.accounts.sol_recieving_account.try_borrow_mut_lamports()? += lamports;
        Ok(())
    }
}
#[error_code]
pub enum CustomError {
    #[msg("No fees to withdraw")]
    NoFeesToWithdraw,
    #[msg("Invalid signer")]
    InvalidSigner,
    #[msg("Curve is being migrated")]
    CurveInMigration
}
#[event]
pub struct InitializeMigrateEvent {
    mint: Pubkey,
}
#[account]
pub struct LinearBondingCurve {
    pub token: u64,
    pub closed: bool,
}
impl LinearBondingCurve {
    pub fn buy(&mut self, token_amount: u64) -> u64 {
        // if you buy token_amount tokens, return the amount of lamports you will need
       let result = self.discrete_integral(self.token, self.token + token_amount);
        self.token += token_amount;
        return (result / DENOMINATOR_C) + 1;
    }
    pub fn sell(&mut self, token_amount: u64) -> u64 {
        // if you sell token_amount tokens, return the amount of lamports you get
        let result = self.discrete_integral(self.token - token_amount, self.token);
        self.token -= token_amount;
        return (result / DENOMINATOR_C) + 1;
    }
    fn discrete_integral(&self, a: u64, b: u64) -> u64 {
        let temp = b + 1 - a;
        let times = temp / 2;
        let sum = a + b;
        let bit = (temp % 2) * sum / 2;
        return times * sum + bit;
    }
    pub fn mkt_cap(&self) -> u64 {
        return self.token / DENOMINATOR_C * TOKEN_SUPPLY
    }
}
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        seeds = [b"auth"],
        bump,
        payer = signer,
        space = 8,
    )]
    /// CHECK: 
    pub program_authority: AccountInfo<'info>,
    #[account(
        init,
        seeds = [b"sol"],
        bump,
        payer = signer,
        space = 8,
    )]
    /// CHECK: 
    pub program_sol_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>
}

#[derive(Accounts)]
pub struct CreateTokenAndBuy<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        init,
        payer = signer,
        mint::authority = program_authority,
        mint::decimals = MINT_DECIMALS,
    )]
    pub mint: Account<'info, Mint>,
    #[account(
        init,
        payer = signer,
        associated_token::mint = mint,
        associated_token::authority = signer,
    )]
    pub signer_token_account: Account<'info, TokenAccount>,
    #[account(
        init,
        seeds = [b"curve", mint.key().as_ref()],
        bump,
        payer = signer,
        space = 8 + 8 + 1
    )]
    pub bonding_curve: Account<'info, LinearBondingCurve>,
    #[account(
        init,
        payer = signer,
        token::mint = mint,
        token::authority = program_authority,
        seeds = [b"token", mint.key().as_ref()],
        bump,
    )]
    pub curve_token_account: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = signer,
        seeds = [b"sol", mint.key().as_ref()],
        bump,
        space = 8,
    )]
    /// CHECK: 
    pub curve_sol_account: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [b"sol"],
        bump
    )]
    pub program_sol_account: AccountInfo<'info>,
    #[account(
        seeds = [b"auth"],
        bump,
    )]
    /// CHECK: 
    pub program_authority: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
}

#[derive(Accounts)]
pub struct Buy<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub signer_token_account: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [b"curve", mint.key().as_ref()],
        bump,
    )]
    pub bonding_curve: Account<'info, LinearBondingCurve>,
    #[account(
        mut,
        seeds = [b"token", mint.key().as_ref()],
        bump,
    )]
    pub curve_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"sol", mint.key().as_ref()],
        bump,
    )]
    /// CHECK: 
    pub curve_sol_account: AccountInfo<'info>,
    #[account(
        seeds = [b"auth"],
        bump,
    )]
    /// CHECK: 
    pub program_authority: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [b"sol"],
        bump
    )]
    pub program_sol_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>
}
#[derive(Accounts)]
pub struct Swap<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    pub from_mint: Account<'info, Mint>,
    pub to_mint: Account<'info, Mint>,
    #[account(mut)]
    pub signer_from_token_account: Account<'info, TokenAccount>,
    #[account(mut)]
    pub signer_to_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"curve", from_mint.key().as_ref()],
        bump,
    )]
    pub from_curve: Account<'info, LinearBondingCurve>,
    #[account(
        mut,
        seeds = [b"curve", to_mint.key().as_ref()],
        bump,
    )]
    pub to_curve: Account<'info, LinearBondingCurve>,
    #[account(
        mut,
        seeds = [b"token", from_mint.key().as_ref()],
        bump,
    )]
    pub from_curve_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"token", to_mint.key().as_ref()],
        bump,
    )]
    pub to_curve_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"sol", from_mint.key().as_ref()],
        bump,
    )]
    /// CHECK: 
    pub from_curve_sol_account: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [b"sol", from_mint.key().as_ref()],
        bump,
    )]
    /// CHECK: 
    pub to_curve_sol_account: AccountInfo<'info>,
    #[account(
        seeds = [b"auth"],
        bump
    )]
    // CHECK: 
    pub program_authority: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [b"sol"],
        bump
    )]
    pub program_sol_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
pub struct Sell<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(mut)]
    pub signer_token_account: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [b"curve", mint.key().as_ref()],
        bump,
    )]
    pub bonding_curve: Account<'info, LinearBondingCurve>,
    #[account(
        mut,
        seeds = [b"token", mint.key().as_ref()],
        bump,
    )]
    pub curve_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"sol", mint.key().as_ref()],
        bump,
    )]
    /// CHECK: 
    pub curve_sol_account: AccountInfo<'info>,
    #[account(
        seeds = [b"auth"],
        bump,
    )]
    /// CHECK: 
    pub program_authority: AccountInfo<'info>,
    #[account(
        mut,
        seeds = [b"sol"],
        bump
    )]
    pub program_sol_account: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>
}
#[derive(Accounts)]
pub struct WithdrawFees<'info> {
    #[account(mut)]
    pub signer: Signer<'info>,
    #[account(
        mut,
        seeds = [b"sol"],
        bump,
    )]
    /// CHECK: 
    pub program_sol_account: AccountInfo<'info>,
}
#[derive(Accounts)]
pub struct Migrate<'info> {
    pub signer: Signer<'info>,
    #[account(mut)]
    /// CHECK: 
    pub sol_recieving_account: AccountInfo<'info>,
    #[account(mut)]
    pub token_recieving_account: Account<'info, TokenAccount>,
    pub mint: Account<'info, Mint>,
    #[account(
        mut,
        seeds = [b"token", mint.key().as_ref()],
        bump,
    )]
    pub curve_token_account: Account<'info, TokenAccount>,
    #[account(
        mut,
        seeds = [b"sol", mint.key().as_ref()],
        bump,
    )]
    /// CHECK: 
    pub curve_sol_account: AccountInfo<'info>,
    #[account(
        seeds = [b"auth"],
        bump,
    )]
    /// CHECK:
    pub program_authority: AccountInfo<'info>,
    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>
}