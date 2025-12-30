use anchor_lang::prelude::*;
use anchor_spl::{
    token::{Token, TokenAccount, Mint, Transfer, transfer},
    associated_token::AssociatedToken,    
};

use anchor_lang::{
    Discriminator,
    solana_program::sysvar::instructions::{
        ID as INSTRUCTIONS_SYSVAR_ID,
        load_current_index_checked,
        load_instruction_at_checked
    }
};

mod state;
mod errors;
use errors::*;

declare_id!("22222222222222222222222222222222222222222222");

#[program]
pub mod flash_loan {
    use super::*;

    pub fn borrow(ctx: Context<Loan>, borrow_amount: u64) -> Result<()> {
        
        // check if borrow amount is greater than 0
        require!(borrow_amount > 0, ProtocolError::InvalidAmount);

        // derive signer seeds for the protocol account necessary to sign tranfer transaction
        let seeds = &[
            b"protocol".as_ref(),
            &[ctx.bumps.protocol]
        ];

        let signer_seeds = &[&seeds[..]];

        // transfer the funds from the protocol to the borrower
        transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                Transfer {
                    from: ctx.accounts.protocol_ata.to_account_info(),
                    to: ctx.accounts.borrower_ata.to_account_info(),
                    authority: ctx.accounts.protocol.to_account_info(),
                },
                signer_seeds,
            ),
            borrow_amount,
        )?;

        // Instruction Introspection to verify repayment instruction
        let ixs = ctx.accounts.instructions.to_account_info();

        // Check if borrow instruction is the first instruction in the transaction.
        let current_index = load_current_index_checked(&ctx.accounts.instructions)?;
        require_eq!(current_index, 0, ProtocolError::InvalidIx); 

        // Check how many instruction we have in this transaction
        let instruction_sysvar = ixs.try_borrow_data()?;
        let len = u16::from_le_bytes(instruction_sysvar[0..2].try_into().unwrap());

        // Ensure we have a repay instruction
        if let Ok(repay_ix) = load_instruction_at_checked(len as usize - 1, &ixs) {

            // Instruction checks
            require_keys_eq!(repay_ix.program_id, ID, ProtocolError::InvalidProgram);
            require!(repay_ix.data[0..8].eq(instruction::Repay::DISCRIMINATOR), ProtocolError::InvalidIx);

            // verify ATA accounts
            require_keys_eq!(repay_ix.accounts.get(3).ok_or(ProtocolError::InvalidBorrowerAta)?.pubkey, ctx.accounts.borrower_ata.key(), ProtocolError::InvalidBorrowerAta);
            require_keys_eq!(repay_ix.accounts.get(4).ok_or(ProtocolError::InvalidProtocolAta)?.pubkey, ctx.accounts.protocol_ata.key(), ProtocolError::InvalidProtocolAta);

        } else {
            return Err(ProtocolError::MissingRepayIx.into());
        }


        Ok(())
    }

    pub fn repay(ctx: Context<Loan>) -> Result<()> {

        
        let ixs = ctx.accounts.instructions.to_account_info();

        let mut amount_borrowed: u64;

        if let Ok(borrow_ix) = load_instruction_at_checked(0, &ixs) {
            
            // Check the amount borrowed:
            let mut borrowed_data: [u8;8] = [0u8;8];
            borrowed_data.copy_from_slice(&borrow_ix.data[8..16]);
            amount_borrowed = u64::from_le_bytes(borrowed_data)

        } else {
            return Err(ProtocolError::MissingBorrowIx.into());
        }

        // Add the fee to the amount borrowed (hardcoded to 500 basis point)
        let fee = (amount_borrowed as u128).checked_mul(500).unwrap().checked_div(10_000).ok_or(ProtocolError::Overflow)? as u64;
        amount_borrowed = amount_borrowed.checked_add(fee).ok_or(ProtocolError::Overflow)?;

        // Transfer the funds from the protocol to the borrower
        transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(), 
                Transfer {
                    from: ctx.accounts.borrower_ata.to_account_info(),
                    to: ctx.accounts.protocol_ata.to_account_info(),
                    authority: ctx.accounts.borrower.to_account_info(),
                }
            ), 
            amount_borrowed
        )?;

        Ok(())
    } 

}

#[derive(Accounts)]
pub struct Loan<'info> {

    #[account(mut)]
    pub borrower: Signer<'info>, // borrower account

    
    #[account(
        seeds = [b"protocol".as_ref()],
        bump,
    )]
    pub protocol: SystemAccount<'info>, // pda account for protocol

    pub mint: Account<'info, Mint>, // mint account

    #[account(
        init_if_needed, // only initialize account if borrower doesn't have one yet
        payer = borrower,
        associated_token::mint = mint,
        associated_token::authority = borrower,
    )]
    pub borrower_ata: Account<'info, TokenAccount>, // ATA account needed for borrower to hold mint account

    #[account(
        mut, 
        associated_token::mint = mint,
        associated_token::authority = protocol,
    )]
    pub protocol_ata: Account<'info, TokenAccount>, // ATA account needed for protocol to hold mint account

    #[account(address = INSTRUCTIONS_SYSVAR_ID)]
    /// CHECK: InstructionSysvar account
    instructions: UncheckedAccount<'info>,

    pub token_program: Program<'info, Token>,
    pub associated_token_program: Program<'info, AssociatedToken>,
    pub system_program: Program<'info, System>,
}

