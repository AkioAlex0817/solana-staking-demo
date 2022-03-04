use std::convert::TryInto;
use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount};

declare_id!("5XDKq2QwrKwkJuBKR3udSDW3NjkXsBjUMZ6ZBLEA7CVn");

pub mod constants {
    pub const X_TOKEN_MINT_PUBKEY: &str = "AhhDdiazhaCepo3awLUtUNkRy6hYmVM6A6SPHZ1qPXn3";
    pub const Y_TOKEN_MINT_PUBKEY: &str = "FNWakZbe1HHA9jgWzQegrDW4QLXDLjKCUSLTvsF1QzZj";
}

#[program]
pub mod alex_solana_staking {
    use anchor_spl::token;
    use solana_program::entrypoint::ProgramResult;
    use super::*;

    pub fn initialize(_ctx: Context<Initialize>, _nonce: u8) -> ProgramResult { Ok(()) }

    pub fn stake(ctx: Context<Stake>, nonce: u8, amount: u64) -> ProgramResult {
        let total_pool_token = ctx.accounts.token_pool.amount;
        let total_y_token = ctx.accounts.y_token_mint.supply;

        let x_token_mint_key: Pubkey = ctx.accounts.x_token_mint.key();
        let seeds = &[x_token_mint_key.as_ref(), &[nonce]];
        let signer = [&seeds[..]];

        if total_pool_token == 0 || total_y_token == 0 {
            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::MintTo {
                    mint: ctx.accounts.y_token_mint.to_account_info(),
                    to: ctx.accounts.receiver.to_account_info(),
                    authority: ctx.accounts.token_pool.to_account_info(),
                },
                &signer,
            );
            token::mint_to(cpi_ctx, amount);
        } else {
            let what: u64 = (amount as u128)
                .checked_mul(total_y_token as u128)
                .unwrap()
                .checked_div(total_pool_token as u128)
                .unwrap()
                .try_into()
                .unwrap();

            let cpi_ctx = CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                token::MintTo {
                    mint: ctx.accounts.y_token_mint.to_account_info(),
                    to: ctx.accounts.receiver.to_account_info(),
                    authority: ctx.accounts.token_pool.to_account_info(),
                },
                &signer,
            );
            token::mint_to(cpi_ctx, what)?;
        }

        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.sender.to_account_info(),
                to: ctx.accounts.token_pool.to_account_info(),
                authority: ctx.accounts.sender_authority.to_account_info(),
            },
        );

        token::transfer(cpi_ctx, amount)?;

        Ok(())
    }

    pub fn unstake(ctx: Context<Unstake>, nonce: u8, amount: u64) -> ProgramResult {
        let total_pool_token = ctx.accounts.token_pool.amount;
        let total_y_token = ctx.accounts.y_token_mint.supply;

        //burn
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Burn {
                mint: ctx.accounts.y_token_mint.to_account_info(),
                to: ctx.accounts.withdraw_token.to_account_info(),
                authority: ctx.accounts.withdraw_token_authority.to_account_info(),
            },
        );
        token::burn(cpi_ctx, amount)?;

        let what: u64 = (amount as u128)
            .checked_mul(total_pool_token as u128)
            .unwrap()
            .checked_div(total_y_token as u128)
            .unwrap()
            .try_into()
            .unwrap();
        let x_token_mint_key = ctx.accounts.x_token_mint.key();
        let seeds = &[x_token_mint_key.as_ref(), &[nonce]];
        let signer = &[&seeds[..]];

        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.token_pool.to_account_info(),
                to: ctx.accounts.receive_token.to_account_info(),
                authority: ctx.accounts.token_pool.to_account_info(),
            },
            signer,
        );
        token::transfer(cpi_ctx, what)?;

        Ok(())
    }
}

const E0: u128 = 1;

pub fn get_price<'info>(
    pool: &Account<'info, TokenAccount>,
    mint: &Account<'info, Mint>,
) -> (u64, String) {
    let total_token = pool.amount;
    let total_x_token = mint.supply;
    if total_x_token == 0 {
        return (0, String::from("0"));
    }
    let price_uint = (total_token as u128)
        .checked_mul(E0 as u128)
        .unwrap()
        .checked_div(total_x_token as u128)
        .unwrap()
        .try_into()
        .unwrap();
    let price_float = (total_token as f64) / (total_x_token as f64);
    (price_uint, price_float.to_string())
}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
    address = constants::X_TOKEN_MINT_PUBKEY.parse::< Pubkey > ().unwrap(),
    )]
    pub x_token_mint: Box<Account<'info, Mint>>,

    #[account(
    init,
    payer = initializer,
    token::mint = x_token_mint,
    token::authority = token_pool,
    seeds = [constants::X_TOKEN_MINT_PUBKEY.parse::< Pubkey > ().unwrap().as_ref()],
    bump,
    )]
    pub token_pool: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub initializer: Signer<'info>,

    pub system_program: Program<'info, System>,
    pub token_program: Program<'info, Token>,
    pub rent: Sysvar<'info, Rent>,
}

#[derive(Accounts)]
#[instruction(nonce: u8)]
pub struct Stake<'info> {
    #[account(
    address = constants::X_TOKEN_MINT_PUBKEY.parse::< Pubkey > ().unwrap(),
    )]
    pub x_token_mint: Box<Account<'info, Mint>>,

    #[account(
    mut,
    address = constants::Y_TOKEN_MINT_PUBKEY.parse::< Pubkey > ().unwrap(),
    )]
    pub y_token_mint: Box<Account<'info, Mint>>,

    #[account(mut)]
    pub sender: Box<Account<'info, TokenAccount>>,

    pub sender_authority: Signer<'info>,

    #[account(
    mut,
    seeds = [x_token_mint.key().as_ref()],
    bump = nonce,
    )]
    pub token_pool: Box<Account<'info, TokenAccount>>,

    #[account(mut)]
    pub receiver: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

#[derive(Accounts)]
#[instruction(nonce: u8)]
pub struct Unstake<'info> {
    #[account(
    address = constants::X_TOKEN_MINT_PUBKEY.parse::< Pubkey > ().unwrap(),
    )]
    pub x_token_mint: Box<Account<'info, Mint>>,

    #[account(
    mut,
    address = constants::Y_TOKEN_MINT_PUBKEY.parse::< Pubkey > ().unwrap(),
    )]
    pub y_token_mint: Box<Account<'info, Mint>>,

    #[account(mut)]
    pub withdraw_token: Box<Account<'info, TokenAccount>>,

    pub withdraw_token_authority: Signer<'info>,

    #[account(
    mut,
    seeds = [x_token_mint.key().as_ref()],
    bump = nonce,
    )]
    pub token_pool: Box<Account<'info, TokenAccount>>,
    #[account(mut)]
    pub receive_token: Box<Account<'info, TokenAccount>>,

    pub token_program: Program<'info, Token>,
}

#[event]
pub struct LogHandler {
    amount: u64,
}

