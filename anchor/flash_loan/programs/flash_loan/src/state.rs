use anchor_lang::prelude::*;
 
#[derive(InitSpace)]
#[account]
pub struct Loan {
    pub borrower: Pubkey,
    pub mint: Pubkey,
    pub amount: u64,
    pub fee: u64,
    pub bump: u8,
}