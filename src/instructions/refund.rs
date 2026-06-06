use pinocchio::{
    AccountView, ProgramResult, cpi::{Seed, Signer}, error::ProgramError, sysvars::{Sysvar, clock::Clock}
};
use pinocchio_pubkey::derive_address;

use crate::{state::{Fundraiser, Contributor}, SECONDS_TO_DAYS};

pub fn process_refund_instruction(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [
        contributor,
        maker,
        mint_to_raise,
        fundraiser,
        contributor_account,
        contributor_ata,
        vault,
        _system_program,
        _token_program,
    ] = accounts 
    else {
        return Err(ProgramError::NotEnoughAccountKeys)
    };

    {
        let contributor_ata_state = pinocchio_token::state::Account::from_account_view(contributor_ata)?;
        if contributor_ata_state.owner() != contributor.address() {
            return Err(ProgramError::IllegalOwner);
        }
        
        if contributor_ata_state.mint() != mint_to_raise.address() {
            return Err(ProgramError::InvalidAccountData);
        }
        
    }

    let bump = data[0];
    let contributor_bump = data[1];

    let seed = [b"contributor".as_ref(), contributor.address().as_ref(), &[contributor_bump]];

    let contributor_account_pda = derive_address(&seed, None, &crate::ID.to_bytes());
    //assert_eq!(contributor_account_pda, *contributor_account.address().as_array());
    if contributor_account_pda != *contributor_account.address().as_array() {

        return Err(ProgramError::InvalidAccountData);
    }


    let fundraiser_data = Fundraiser::from_account_info(fundraiser)?;

    if fundraiser_data.maker() != maker.address() {
        return Err(ProgramError::InvalidAccountData);
    }

    let contributor_data = Contributor::from_account_info(contributor_account)?;

    let current_time = Clock::get()?.unix_timestamp;
    if fundraiser_data.duration < ((current_time - fundraiser_data.time_started())/SECONDS_TO_DAYS) as u8 {
        return Err(ProgramError::InvalidArgument);
    }

    let vault_data = unsafe {vault.borrow_unchecked()};
    let vault_amount = u64::from_le_bytes(vault_data[64..72].try_into().unwrap());

    if vault_amount > fundraiser_data.amount_to_raise() {
        return Err(ProgramError::InvalidArgument);
    }

    let fund_amount = fundraiser_data.current_amount() - contributor_data.amount();
    fundraiser_data.set_current_amount(fund_amount);


    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_array()),
        Seed::from(bump_bytes.as_ref()),
    ];
    let signer = Signer::from(&signer_seeds);


    pinocchio_token::instructions::Transfer::new(vault, contributor_ata, fundraiser, contributor_data.amount())
        .invoke_signed(&[signer])?;

    let contributor_account_lamports = contributor_account.lamports();


    contributor.set_lamports(contributor.lamports() + contributor_account_lamports);
    contributor_account.set_lamports(0);


    let _ = contributor_account.close();

    Ok(())
}