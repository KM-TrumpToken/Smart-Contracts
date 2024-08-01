use anchor_lang::prelude::*;
use anchor_lang::solana_program::entrypoint::ProgramResult;
use anchor_spl::token::{self, Mint, Token, TokenAccount};
use pyth_sdk_solana::{load_price_feed_from_account_info, PriceFeed};
use std::str::FromStr;

declare_id!("BJGmSiManG1xTYByov97DAsiB5yNF59gdqSHzo8URq9E");

const BTC_USDC_FEED: &str = "J83w4HKfqxwcq3BEMMkPFSppX3gqekLyLJBexebFVkix";


#[derive(Accounts)]
pub struct FetchSolPrice<'info> {
    /// CHECK:
    pub signer: AccountInfo<'info>,
    /// CHECK:
    #[account(address = Pubkey::from_str(BTC_USDC_FEED).unwrap() @ FeedError::InvalidPriceFeed)]
    pub price_feed: AccountInfo<'info>,
}

#[error_code]
pub enum FeedError {
    #[msg("Invalid Price Feed")]
    InvalidPriceFeed,
}

#[program]
pub mod ico {
    // pub const USDT_MINT_ADDRESS: &str = "2cCcopLR3UAk4WEgLXFteaVvJurctuc25Mx8JEQCQoY7";
    pub const ICO_MINT_ADDRESS: &str = "8jf4rgEzz5Lr2B3XfWnj6cE9bbSzxRg9RuZh21yQE6TL";
    pub const SCALE: u64 = 1_000_000;
    use super::*;

    /* 
    ===========================================================
        create_ico_ata function use CreateIcoATA struct
    ===========================================================
*/
    pub fn create_ico_ata(
        ctx: Context<CreateIcoATA>,
        ico_amount: u64,
        end_time:i64,
        usdt_price: u64,
        usdt_ata_for_admin: Pubkey,
        manager: Pubkey,
        phase: u64
    ) -> Result<()> {
        msg!("create program ATA for hold ICO");
        // transfer ICO admin to program ata
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.ico_ata_for_admin.to_account_info(),
                to: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
                authority: ctx.accounts.admin.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, ico_amount)?;
        msg!("send {} ICO to program ATA.", ico_amount);

        
        let data = &mut ctx.accounts.data;
        data.end_time = end_time;
        data.usdt = usdt_price;
        data.total_amount = ico_amount;
        data.amount_sold = 0;
        data.admin = *ctx.accounts.admin.key;
        data.manager = manager;
        data.usdt_ata_for_admin = usdt_ata_for_admin;
        data.phase_id = phase;
        msg!("save data in program PDA.");
        Ok(())
    }

    /* 
    ===========================================================
        deposit_ico_in_ata function use DepositIcoInATA struct
    ===========================================================
*/
    pub fn deposit_ico_in_ata(ctx: Context<DepositIcoInATA>, ico_amount: u64) -> ProgramResult {
        if ctx.accounts.data.admin != *ctx.accounts.admin.key {
            return Err(ProgramError::IncorrectProgramId);
        }
        // transfer ICO admin to program ata
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.ico_ata_for_admin.to_account_info(),
                to: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
                authority: ctx.accounts.admin.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, ico_amount)?;
        let data = &mut ctx.accounts.data;
        data.total_amount = ico_amount;
        msg!("deposit {} ICO in program ATA.", ico_amount);
        Ok(())
    }

    /* 
    ===========================================================
        buy_with_sol function use BuyWithSol struct
    ===========================================================
    */
  




    pub fn update_admin(ctx: Context<UpdateAdmin>, usdt_ata_for_admin:Pubkey,new_admin: Pubkey, new_manager:Pubkey) -> ProgramResult {
        // if ctx.accounts.data.manager != *ctx.accounts.manager.key {
        //     return Err(ProgramError::IncorrectProgramId);
        // }
        if ctx.accounts.data.admin != *ctx.accounts.admin.key {
            return Err(ProgramError::IncorrectProgramId);
        }
        let data = &mut ctx.accounts.data;
       
        
        data.admin = new_admin;
        data.usdt_ata_for_admin = usdt_ata_for_admin;
        data.manager = new_manager;
        
        Ok(())
    }


    /* 
    -----------------------------------------------------------
        CreateIcoATA struct for create_ico_ata function
    -----------------------------------------------------------
*/
    #[derive(Accounts)]
    pub struct CreateIcoATA<'info> {
        // 1. PDA (pubkey) for ico ATA for our program.
        // seeds: [ico_mint + current program id] => "HashMap[seeds+bump] = pda"
        // token::mint: Token Program wants to know what kind of token this ATA is for
        // token::authority: It's a PDA so the authority is itself!

        #[account(
        init,
        payer = admin,
        seeds = [ ICO_MINT_ADDRESS.parse::<Pubkey>().unwrap().as_ref() ],
        bump,
        token::mint = ico_mint,
        token::authority = ico_ata_for_ico_program,
    )]
        pub ico_ata_for_ico_program: Account<'info, TokenAccount>,

        #[account(init, payer=admin, space=9000, seeds=[b"data", admin.key().as_ref()], bump)]
        pub data: Account<'info, Data>,

        #[account(
        address = ICO_MINT_ADDRESS.parse::<Pubkey>().unwrap(),
    )]
        pub ico_mint: Account<'info, Mint>,

        #[account(mut)]
        pub ico_ata_for_admin: Account<'info, TokenAccount>,

        #[account(mut)]
        pub admin: Signer<'info>,

        pub system_program: Program<'info, System>,
        pub token_program: Program<'info, Token>,
        pub rent: Sysvar<'info, Rent>,
    }

    /* 
    -----------------------------------------------------------
        DepositIcoInATA struct for deposit_ico_in_ata function
    -----------------------------------------------------------
*/
    #[derive(Accounts)]
    pub struct DepositIcoInATA<'info> {
        #[account(mut)]
        pub ico_ata_for_ico_program: Account<'info, TokenAccount>,

        #[account(mut)]
        pub data: Account<'info, Data>,

        #[account(
        address = ICO_MINT_ADDRESS.parse::<Pubkey>().unwrap(),
    )]
        pub ico_mint: Account<'info, Mint>,

        #[account(mut)]
        pub ico_ata_for_admin: Account<'info, TokenAccount>,

        #[account(mut)]
        pub admin: Signer<'info>,
        pub token_program: Program<'info, Token>,
    }

    /* 
    -----------------------------------------------------------
        BuyWithSol struct for buy_with_sol function
    -----------------------------------------------------------
*/
    #[derive(Accounts)]
    #[instruction(_ico_ata_for_ico_program_bump: u8)]
    pub struct BuyWithSol<'info> {
        #[account(
        mut,
        seeds = [ ico_mint.key().as_ref() ],
        bump = _ico_ata_for_ico_program_bump,
    )]
        pub ico_ata_for_ico_program: Account<'info, TokenAccount>,

        #[account(mut)]
        pub data: Account<'info, Data>,

        #[account(
        address = ICO_MINT_ADDRESS.parse::<Pubkey>().unwrap(),
    )]
        pub ico_mint: Account<'info, Mint>,

        #[account(mut)]
        pub ico_ata_for_user: Account<'info, TokenAccount>,
         
        /// CHECK:
        #[account(address = Pubkey::from_str(BTC_USDC_FEED).unwrap() @ FeedError::InvalidPriceFeed)]
        pub price_feed: AccountInfo<'info>,

        #[account(mut)]
        pub user: Signer<'info>,
        pub manager:Signer<'info>,

        /// CHECK:
        #[account(mut)]
        pub admin: AccountInfo<'info>,

        pub token_program: Program<'info, Token>,
        pub system_program: Program<'info, System>,
    }


   
     
     #[derive(Accounts)]
    pub struct UpdateAdmin<'info> {
        #[account(mut)]
        pub usdt_ata_for_admin: Account<'info, TokenAccount>,
        #[account(mut)]
        pub data: Account<'info, Data>,
        #[account(mut)]
        pub admin: Signer<'info>,
        pub system_program: Program<'info, System>,
    }
    /* 
    -----------------------------------------------------------
        Data struct for PDA Account
    -----------------------------------------------------------
*/
    #[account]
    pub struct Data {
        pub phase_id: u64,
        pub end_time: i64,
        pub amount_sold: u64,
        pub total_amount:u64,
        pub usdt: u64,
        pub admin: Pubkey,
        pub manager: Pubkey,
        pub usdt_ata_for_admin: Pubkey,
    }
}