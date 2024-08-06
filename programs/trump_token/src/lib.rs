use anchor_lang::prelude::*;
use anchor_lang::solana_program::entrypoint::ProgramResult;
use anchor_spl::token::{self, Mint, Token, TokenAccount};
use pyth_sdk_solana::{load_price_feed_from_account_info, PriceFeed};
use std::str::FromStr;

declare_id!("BJGmSiManG1xTYByov97DAsiB5yNF59gdqSHzo8URq9E");

const SOL_USDC_FEED: &str = "";


#[derive(Accounts)]
pub struct FetchSolPrice<'info> {
    /// CHECK:
    pub signer: AccountInfo<'info>,
    /// CHECK:
    #[account(address = Pubkey::from_str(SOL_USDC_FEED).unwrap() @ FeedError::InvalidPriceFeed)]
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
    pub fn buy_with_sol(
        ctx: Context<BuyWithSol>,
        _ico_ata_for_ico_program_bump: u8,
        sol_amount: u64,
    ) -> ProgramResult {
        // transfer sol from user to admin
        const STALENESS_THRESHOLD : u64 = 60; // staleness threshold in seconds
        let price_account_info = &ctx.accounts.price_feed;
        let data = &mut ctx.accounts.data;
        let price_feed: PriceFeed = load_price_feed_from_account_info( &price_account_info ).unwrap();
        let current_timestamp = Clock::get()?.unix_timestamp;
        // let current_timestamp1 = Clock::get()?.unix_timestamp;
        if data.end_time < Clock::get()?.unix_timestamp {
            return Err(ProgramError::InvalidArgument);
        }
        if data.admin != ctx.accounts.admin.key() {
            return Err(ProgramError::IllegalOwner);
        }
       
        let current_price = price_feed.get_price_no_older_than(current_timestamp, STALENESS_THRESHOLD);
        match current_price {
            Some(price) => {
              let display_price = u64::try_from(price.price).unwrap() / 10u64.pow(u32::try_from(-price.expo).unwrap());
              msg!("Price using match: {:?}", display_price);
              let amount_in_usdt = display_price * SCALE;
            //   let sol_amountWithScale = sol_amount / 1000;
              let sol_in_usdt = amount_in_usdt * sol_amount;
              let ico_amount;
                ico_amount = sol_in_usdt / data.usdt;
            if ico_amount > (data.total_amount - data.amount_sold) {
                return Err(ProgramError::InsufficientFunds);
            }
             
            let funding_amount = sol_amount / 2;
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
            token::transfer(cpi_ctx, ico_amount)?;
            data.amount_sold = data.amount_sold + ico_amount;
            msg!("transfer {} ico to buyer/user.", ico_amount);
        },
        None => msg!("No price found"),
        }
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
        if ctx.accounts.data.end_time < Clock::get()?.unix_timestamp {
            return Err(ProgramError::IncorrectProgramId);
        }
        if ctx.accounts.data.usdt_ata_for_admin != ctx.accounts.usdt_ata_for_admin.key() || 
        ctx.accounts.data.usdt_ata_for_funding_account != ctx.accounts.usdt_ata_for_funding_account.key(){
            return Err(ProgramError::IllegalOwner);
        }

        // amount calculation for admin and funding account
        let amount_share = usdt_amount/2;

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
        let mut ico_amount =0;
             ico_amount = usdt_amount / ctx.accounts.data.usdt;
             ico_amount = ico_amount * 1000000000; 
        let data = &mut ctx.accounts.data;
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
        
        data.amount_sold = data.amount_sold + ico_amount;
        msg!("transfer {} ico to buyer/user.", ico_amount);
        Ok(())
    }


    /* 
    ===========================================================
        buy_with_usdt function use BuyWithUsdc struct
    ===========================================================
*/
pub fn buy_with_usdc(
    ctx: Context<BuyWithUsdc>,
    _ico_ata_for_ico_program_bump: u8,
    usdt_amount: u64,
) -> ProgramResult {
    if ctx.accounts.data.end_time < Clock::get()?.unix_timestamp {
        return Err(ProgramError::IncorrectProgramId);
    }
    if ctx.accounts.data.usdc_ata_for_admin != ctx.accounts.usdc_ata_for_admin.key() ||
    ctx.accounts.data.usdc_ata_for_funding_account != ctx.accounts.usdc_ata_for_funding_account.key() {
        return Err(ProgramError::IllegalOwner);
    }
    

    let amount_share = usdt_amount / 2;
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
    let mut ico_amount =0;
         ico_amount = usdt_amount / ctx.accounts.data.usdt;
         ico_amount = ico_amount * 1000000000; 
    let data = &mut ctx.accounts.data;
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
    
    data.amount_sold = data.amount_sold + ico_amount;
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
    ) -> ProgramResult {
        if ctx.accounts.data.admin != ctx.accounts.admin.key() {
            return Err(ProgramError::IllegalOwner);
        }
        if ctx.accounts.data.manager != ctx.accounts.manager.key() {
            return Err(ProgramError::IllegalOwner);
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
        token::transfer(cpi_ctx, token_amount)?;
        Ok(())
    }

    /* 
    ===========================================================
        update_data function use UpdateData struct
    ===========================================================
*/
    pub fn update_data(ctx: Context<UpdateData>, total_amount: u64,end_time: i64, usdt_price: u64,phase: u64) -> ProgramResult {
        if ctx.accounts.data.manager != *ctx.accounts.manager.key {
            return Err(ProgramError::IncorrectProgramId);
        }
        let data = &mut ctx.accounts.data;
       
        data.end_time = end_time;
        data.usdt = usdt_price;
        data.total_amount = data.total_amount + total_amount - data.amount_sold;
        data.amount_sold = 0;
        data.phase_id = phase;
        msg!("update ico time {}, {} and USDT/ICO {}", total_amount,end_time, usdt_price);
        Ok(())
    }


    pub fn update_admin(ctx: Context<UpdateAdmin>, usdt_ata_for_admin:Pubkey,new_admin: Pubkey, new_manager:Pubkey, usdc_ata_for_admin:Pubkey) -> ProgramResult {
        // if ctx.accounts.data.manager != *ctx.accounts.manager.key {
        //     return Err(ProgramError::IncorrectProgramId);
        // }
        if ctx.accounts.data.admin != *ctx.accounts.admin.key {
            return Err(ProgramError::IncorrectProgramId);
        }
        let data = &mut ctx.accounts.data;
       
        
        data.admin = new_admin;
        data.usdt_ata_for_admin = usdt_ata_for_admin;
        data.usdc_ata_for_admin = usdc_ata_for_admin;
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
        #[account(address = Pubkey::from_str(SOL_USDC_FEED).unwrap() @ FeedError::InvalidPriceFeed)]
        pub price_feed: AccountInfo<'info>,

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
        pub manager:Signer<'info>,

        

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
        pub manager: Signer<'info>,
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
        pub usdc_ata_for_admin: Pubkey,
        pub funding_account: Pubkey,
        pub usdt_ata_for_funding_account: Pubkey,
        pub usdc_ata_for_funding_account: Pubkey
    }
}