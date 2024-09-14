use anchor_lang::prelude::*;
use anchor_lang::solana_program::entrypoint::ProgramResult;
use anchor_spl::token::{self, Mint, Token, TokenAccount};
use pyth_solana_receiver_sdk::price_update::{get_feed_id_from_hex, PriceUpdateV2};
use std::str::FromStr;

declare_id!("HXCVfLkfbXnZXst6HL5G1pDWPdEtxXhprWyFkJmwSPFS");

const SOL_USD_FEED: &str = "7UVimffxr9ow1uXYxsr4LHAcV58mLzhmwaeKvJ1pjLiE";
pub const MAXIMUM_AGE: u64 = 60; // One minute
pub const FEED_ID: &str = "0xef0d8b6fda2ceba41da15d4095d1da392a0d2f8ed0c6c7bc0f4cfac8c280b56d"; // SOL/USD price feed id from https://pyth.network/developers/price-feed-ids

#[derive(Accounts)]
pub struct FetchSolPrice<'info> {
    /// CHECK:
    pub signer: AccountInfo<'info>,
    /// CHECK:
    #[account(address = Pubkey::from_str(SOL_USD_FEED).unwrap() @ FeedError::InvalidPriceFeed)]
    pub price_feed: AccountInfo<'info>,
}

#[error_code]
pub enum FeedError {
    #[msg("Invalid Price Feed")]
    InvalidPriceFeed,
}

#[error_code]
pub enum OwnerError {
    #[msg("Invalid Owner")]
    InvalidOwner,
}
#[error_code]
pub enum IcoTimeError {
    #[msg("The event has not ended yet.")]
    EventNotEnded,
}



#[program]
pub mod ico {
    pub const OWNER_ADDRESS: &str = "EvKCj62U6fsDJyqwcSeanyeK7YUvWeW989vLhksDGa2i";
    pub const ICO_MINT_ADDRESS: &str = "3NqeVUbz469hmNaPBfAKCejJMUkmyj8TwGm1cPZptRFY";
    pub const SCALE: u64 = 1_000_000;
    pub const SCALE_FACTOR_TRUMP_TOKEN: u64 = 100_000_000;
    

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
        usd_price: u64,
        funding_share: u64,
        admin: Pubkey,
        usdt_ata_for_admin: Pubkey,
        usdc_ata_for_admin: Pubkey,
        funding_account: Pubkey,
        usdt_ata_for_funding_account: Pubkey,
        usdc_ata_for_funding_account: Pubkey
    ) -> Result<()> {
        let admin_pubkey = Pubkey::from_str(OWNER_ADDRESS).unwrap();
        // address = Pubkey::from_str(OWNER_ADDRESS).unwrap() @ OwnerError::InvalidOwner
        require!(
            ctx.accounts.admin.key() == admin_pubkey,
            OwnerError::InvalidOwner
        );
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
        data.usd = usd_price;
        data.total_amount = ico_amount;
        data.amount_sold = 0;
        data.funding_share = funding_share;
        data.admin = admin;
        data.usdt_ata_for_admin = usdt_ata_for_admin;
        data.usdc_ata_for_admin = usdc_ata_for_admin;
        data.funding_account = funding_account;
        data.usdt_ata_for_funding_account = usdt_ata_for_funding_account;
        data.usdc_ata_for_funding_account = usdc_ata_for_funding_account;
        msg!("save data in program PDA.");
        Ok(())
    }


    /* 
    ===========================================================
        deposit_ico_in_ata function use DepositIcoInATA struct
    ===========================================================
*/
    pub fn deposit_ico_in_ata(ctx: Context<DepositIcoInATA>, ico_amount: u64) -> ProgramResult {
        if ctx.accounts.data.admin != ctx.accounts.admin.key() {
            return Err(ProgramError::IllegalOwner);
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
        data.total_amount += ico_amount;
        msg!("deposit {} ICO in program ATA.", ico_amount);
        Ok(())
    }

    /* 
    ===========================================================
        buy_with_sol function use BuyWithSol struct
    ===========================================================
*/
    pub fn buy_with_sol(
        ctx: Context<BuyWithSol>,
        _ico_ata_for_ico_program_bump: u8,
        sol_amount: u64
    ) -> ProgramResult {

        // transfer sol from user to admin
        let data = &mut ctx.accounts.data;
        
        let price_update = &mut ctx.accounts.price_feed;
        let price = price_update.get_price_no_older_than(
            &Clock::get()?,
            MAXIMUM_AGE,
            &get_feed_id_from_hex(FEED_ID).map_err(|_err| ProgramError::Custom(1))?,
        ).map_err(|_err| ProgramError::Custom(1))?;
        // let current_timestamp1 = Clock::get()?.unix_timestamp;
        if data.end_time < Clock::get()?.unix_timestamp {
            return Err(ProgramError::Custom(data.end_time as u32));
        }
        if data.admin != ctx.accounts.admin.key() || data.funding_account != ctx.accounts.funding_account.key() {
            return Err(ProgramError::IllegalOwner);
        }
        
        let pyth_price = u64::try_from(price.price).unwrap();
        //u64::try_from(price.price).unwrap() / 10u64.pow(u32::try_from(-price.exponent).unwrap());

            msg!("Price using match: {:?}", pyth_price);
            let amount_in_usd = pyth_price * SCALE;
            msg!("amount_in_usd: {:?}", amount_in_usd);
            msg!("sol_amount: {:?}", sol_amount);

            //   let sol_amountWithScale = sol_amount / 1000;
            // sol_amount will always be in lamports ( * 1e9 ) 
            // if we will multiply pyth_price with scale then total decimals with pyth_price, scale and 
            // solana lamport will be (9 + 6 + pyth_exponent = 15 + pyth_exponent) 
            let sol_in_usd:u128 = amount_in_usd as u128 * sol_amount as u128;
            msg!("sol_in_usd: {:?}", sol_in_usd);
            // data.usd amount will be scaled with 6 decimal places to handle floating point
            // this calculation used to handle Price Calculation Precision Loss
            // so when we divide sol_in_usd by data.amount then remaining values will be in 9 decimal places and pyth_exponent
        
            let ico_amount_without_exponent_scaling = sol_in_usd / data.usd as u128;
            msg!("ico_amount_without_exponent_scaling: {:?}", ico_amount_without_exponent_scaling);
            // Now we have to divide the ico_amount_without_exponent_scaling amount by pyth exponent to go to 9 decimals places
            // still 1 extra decimals point exist with respect to trump tokens decimals.
            
            let ico_amount_with_extra_decimals_points = ico_amount_without_exponent_scaling / 10u128.pow(u32::try_from(-price.exponent).unwrap());
            msg!("ico_amount_with_extra_decimals_points: {:?}", ico_amount_with_extra_decimals_points);
            // we have to divide the amount by 10
            // Now amount will be with respect to 8 decimals points of trump tokens
            let ico_amount = ico_amount_with_extra_decimals_points / 10;
            msg!("ico_amount: {:?}", ico_amount);

            if ico_amount > (data.total_amount as u128 - data.amount_sold as u128) {
                return Err(ProgramError::InsufficientFunds);
            }
            
            let funding_amount = (sol_amount* data.funding_share)/ 1000;
            let ix = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.admin.key(),
            funding_amount,
             );
            
            anchor_lang::solana_program::program::invoke(
            &ix,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.admin.to_account_info(),
            ],
           )?;
      
           let ix_fund = anchor_lang::solana_program::system_instruction::transfer(
            &ctx.accounts.user.key(),
            &ctx.accounts.funding_account.key(),
            funding_amount,
             );
            
            anchor_lang::solana_program::program::invoke(
            &ix_fund,
            &[
                ctx.accounts.user.to_account_info(),
                ctx.accounts.funding_account.to_account_info(),
            ],
           )?;

           msg!("transfer {} sol to admin.", funding_amount);
           msg!("transfer {} sol to funding account.", funding_amount);

           // transfer ICO from program to user ATA
           // let ico_amount = sol_amount * ctx.accounts.data.sol;
           let ico_mint_address = ctx.accounts.ico_mint.key();
           let seeds = &[ico_mint_address.as_ref(), &[_ico_ata_for_ico_program_bump]];
           let signer = [&seeds[..]];
           let cpi_ctx = CpiContext::new_with_signer(
           ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
                to: ctx.accounts.ico_ata_for_user.to_account_info(),
                authority: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
            },
            &signer,
            );
            token::transfer(cpi_ctx, ico_amount as u64)?;
            data.amount_sold += ico_amount as u64;
            msg!("transfer {} ico to buyer/user.", ico_amount);
     
       
        
        Ok(())
    }

    /* 
    ===========================================================
        buy_with_usdt function use BuyWithUsdt struct
    ===========================================================
*/
    pub fn buy_with_usdt(
        ctx: Context<BuyWithUsdt>,
        _ico_ata_for_ico_program_bump: u8,
        usdt_amount: u64,
    ) -> ProgramResult {
        let data = &mut ctx.accounts.data;
        if data.end_time < Clock::get()?.unix_timestamp {
            return Err(ProgramError::Custom(ctx.accounts.data.end_time as u32));
        }
        
        if data.usdt_ata_for_admin != ctx.accounts.usdt_ata_for_admin.key() || 
        data.usdt_ata_for_funding_account != ctx.accounts.usdt_ata_for_funding_account.key(){
            return Err(ProgramError::IllegalOwner);
        }
        

        // amount calculation for admin and funding account
        let amount_share = (usdt_amount * data.funding_share) / 1000;

        // transfer USDT from user to the admin ATA
        let cpi_ctx = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.usdt_ata_for_user.to_account_info(),
                to: ctx.accounts.usdt_ata_for_admin.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::transfer(cpi_ctx, amount_share)?;
        msg!("transfer {} usdt to admin.", amount_share);


        // transfer USDT from user to the admin ATA
        let cpi_ctx_funding = CpiContext::new(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.usdt_ata_for_user.to_account_info(),
                to: ctx.accounts.usdt_ata_for_funding_account.to_account_info(),
                authority: ctx.accounts.user.to_account_info(),
            },
        );
        token::transfer(cpi_ctx_funding, amount_share)?;
        msg!("transfer {} usdt to funding account.", amount_share);

        // transfer ICO from program to the user ATA
        // let ico_amount = usdt_amount / ctx.accounts.data.usdt;
        let ico_amount = (usdt_amount * SCALE_FACTOR_TRUMP_TOKEN)/ data.usd;
             
        // let data = &mut ctx.accounts.data;
             if ico_amount > (data.total_amount - data.amount_sold) {
                return Err(ProgramError::InsufficientFunds);
            }
        let ico_mint_address = ctx.accounts.ico_mint.key();
        let seeds = &[ico_mint_address.as_ref(), &[_ico_ata_for_ico_program_bump]];
        let signer = [&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
                to: ctx.accounts.ico_ata_for_user.to_account_info(),
                authority: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
            },
            &signer,
        );
        token::transfer(cpi_ctx, ico_amount)?;
        
        data.amount_sold += ico_amount;
        msg!("transfer {} ico to buyer/user.", ico_amount);
        Ok(())
    }


    /* 
    ===========================================================
        buy_with_usdC function use BuyWithUsdc struct
    ===========================================================
*/
pub fn buy_with_usdc(
    ctx: Context<BuyWithUsdc>,
    _ico_ata_for_ico_program_bump: u8,
    usdc_amount: u64,
) -> ProgramResult {
    let data = &mut ctx.accounts.data;
    if data.end_time < Clock::get()?.unix_timestamp {
        return Err(ProgramError::Custom(ctx.accounts.data.end_time as u32));
    }
    if data.usdc_ata_for_admin != ctx.accounts.usdc_ata_for_admin.key() ||
    data.usdc_ata_for_funding_account != ctx.accounts.usdc_ata_for_funding_account.key() {
        return Err(ProgramError::IllegalOwner);
    }
    

    let amount_share = (usdc_amount * data.funding_share) / 1000;
    // transfer USDT from user to the admin ATA
    let cpi_ctx = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        token::Transfer {
            from: ctx.accounts.usdc_ata_for_user.to_account_info(),
            to: ctx.accounts.usdc_ata_for_admin.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        },
    );
    token::transfer(cpi_ctx, amount_share)?;
    msg!("transfer {} usdt to admin.", amount_share);


    // transfer USDT from user to the admin ATA
    let cpi_ctx_funding = CpiContext::new(
        ctx.accounts.token_program.to_account_info(),
        token::Transfer {
            from: ctx.accounts.usdc_ata_for_user.to_account_info(),
            to: ctx.accounts.usdc_ata_for_funding_account.to_account_info(),
            authority: ctx.accounts.user.to_account_info(),
        },
    );
    token::transfer(cpi_ctx_funding, amount_share)?;
    msg!("transfer {} usdt to funding account.", amount_share);


    // transfer ICO from program to the user ATA
    // let ico_amount = usdt_amount / ctx.accounts.data.usdt;
    let ico_amount = (usdc_amount * SCALE_FACTOR_TRUMP_TOKEN) / data.usd;
         
    
         if ico_amount > (data.total_amount - data.amount_sold) {
            return Err(ProgramError::InsufficientFunds);
        }
    let ico_mint_address = ctx.accounts.ico_mint.key();
    let seeds = &[ico_mint_address.as_ref(), &[_ico_ata_for_ico_program_bump]];
    let signer = [&seeds[..]];
    let cpi_ctx = CpiContext::new_with_signer(
        ctx.accounts.token_program.to_account_info(),
        token::Transfer {
            from: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
            to: ctx.accounts.ico_ata_for_user.to_account_info(),
            authority: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
        },
        &signer,
    );
    token::transfer(cpi_ctx, ico_amount)?;
    
    data.amount_sold += ico_amount;
    msg!("transfer {} ico to buyer/user.", ico_amount);
    Ok(())
}

 /* 
    ===========================================================
        Function to Withdraw Remaining token after ICO
    ===========================================================
*/

     pub fn withdraw(
        ctx: Context<WithDraw>,
        _ico_ata_for_ico_program_bump: u8,
        token_amount: u64,
    ) -> Result<()> {
    
        let data = &mut ctx.accounts.data;
        require!(
            data.admin == ctx.accounts.admin.key(),
            OwnerError::InvalidOwner
        );
        // only withdraw function will be executed after ico event has ended
        require!(
            data.end_time < Clock::get()?.unix_timestamp,
            IcoTimeError::EventNotEnded
        );
    
        
        let ico_mint_address = ctx.accounts.ico_mint.key();
        let seeds = &[ico_mint_address.as_ref(), &[_ico_ata_for_ico_program_bump]];
        let signer = [&seeds[..]];
        let cpi_ctx = CpiContext::new_with_signer(
            ctx.accounts.token_program.to_account_info(),
            token::Transfer {
                from: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
                to: ctx.accounts.ico_ata_for_user.to_account_info(),
                authority: ctx.accounts.ico_ata_for_ico_program.to_account_info(),
            },
            &signer,
        );
        token::transfer(cpi_ctx, token_amount)?;
        data.total_amount -= token_amount;
        
        Ok(())
    }

    /* 
    ===========================================================
        update_data function use UpdateData struct
    ===========================================================
*/
    pub fn update_data(ctx: Context<UpdateData>, end_time: i64, usd_price: u64) -> Result<()> {
        // this will be used to extend time if ico event time has ended
        let data = &mut ctx.accounts.data;
        require!(
            data.admin == ctx.accounts.admin.key(),
            OwnerError::InvalidOwner
        );
        require!(
            data.end_time < Clock::get()?.unix_timestamp,
            IcoTimeError::EventNotEnded
        );
        
        
       
        data.end_time = end_time;
        data.usd = usd_price;
        
        msg!("update ico time  {}, and price {}",end_time, usd_price);
        Ok(())
    }


    pub fn update_admin(ctx: Context<UpdateAdmin>, usdt_ata_for_admin:Pubkey,new_admin: Pubkey, usdc_ata_for_admin:Pubkey) -> ProgramResult {
        
        if ctx.accounts.data.admin != ctx.accounts.admin.key() {
            return Err(ProgramError::IllegalOwner);
        }
        let data = &mut ctx.accounts.data;
        data.admin = new_admin;
        data.usdt_ata_for_admin = usdt_ata_for_admin;
        data.usdc_ata_for_admin = usdc_ata_for_admin;
        
        
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

        #[account(init, payer=admin, space=4500, seeds=[b"data", admin.key().as_ref()], bump)]
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
        #[account(address = Pubkey::from_str(SOL_USD_FEED).unwrap() @ FeedError::InvalidPriceFeed)]
        pub price_feed: Account<'info, PriceUpdateV2>,

        #[account(mut)]
        pub user: Signer<'info>,

        /// CHECK:
        #[account(mut)]
        pub admin: AccountInfo<'info>,

        /// CHECK:
        #[account(mut)]
        pub funding_account: AccountInfo<'info>,

        pub token_program: Program<'info, Token>,
        pub system_program: Program<'info, System>,
    }

    /* 
    -----------------------------------------------------------
        BuyWithUsdt struct for buy_with_usdt function
    -----------------------------------------------------------
*/
    #[derive(Accounts)]
    #[instruction(_ico_ata_for_ico_program_bump: u8)]
    pub struct BuyWithUsdt<'info> {
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

        #[account(mut)]
        pub usdt_ata_for_user: Account<'info, TokenAccount>,

        #[account(mut)]
        pub usdt_ata_for_admin: Account<'info, TokenAccount>,

        #[account(mut)]
        pub usdt_ata_for_funding_account:Account<'info, TokenAccount>,

        #[account(mut)]
        pub user: Signer<'info>,

        pub token_program: Program<'info, Token>,
    }

     /* 
    -----------------------------------------------------------
        BuyWithUsdt struct for buy_with_usdt function
    -----------------------------------------------------------
*/
#[derive(Accounts)]
#[instruction(_ico_ata_for_ico_program_bump: u8)]
pub struct BuyWithUsdc<'info> {
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

    #[account(mut)]
    pub usdc_ata_for_user: Account<'info, TokenAccount>,

    #[account(mut)]
    pub usdc_ata_for_admin: Account<'info, TokenAccount>,

    #[account(mut)]
    pub usdc_ata_for_funding_account: Account<'info, TokenAccount>,

    #[account(mut)]
    pub user: Signer<'info>,

    pub token_program: Program<'info, Token>,
}



     #[derive(Accounts)]
    #[instruction(_ico_ata_for_ico_program_bump: u8)]
    pub struct WithDraw<'info> {
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

        #[account(mut)]
        pub admin: Signer<'info>,

        pub token_program: Program<'info, Token>,
    }

    /* 
    -----------------------------------------------------------
        UpdateData struct for update_data function
    -----------------------------------------------------------
*/
    #[derive(Accounts)]
    pub struct UpdateData<'info> {
        #[account(mut)]
        pub data: Account<'info, Data>,
        #[account(mut)]
        pub admin: Signer<'info>,
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
        pub end_time: i64,
        pub amount_sold: u64,
        pub total_amount:u64,
        // funding shares will be the multiple of 10. e.g 1000/1000 will be 100 percent
        pub funding_share: u64,
        pub usd: u64,
        pub admin: Pubkey,
        pub usdt_ata_for_admin: Pubkey,
        pub usdc_ata_for_admin: Pubkey,
        pub funding_account: Pubkey,
        pub usdt_ata_for_funding_account: Pubkey,
        pub usdc_ata_for_funding_account: Pubkey
    }
}