use pinocchio::{
    AccountView, ProgramResult, cpi::{Seed, Signer}, error::ProgramError, sysvars::{Sysvar, rent::Rent}
};
use pinocchio_pubkey::derive_address;
use pinocchio_system::instructions::CreateAccount;

use crate::{state::Fundraiser, MIN_AMOUNT_TO_RAISE};

pub fn process_initialize_instruction(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [
        maker,
        mint_to_raise,
        fundraiser,
        vault,
        system_program,
        token_program,
        _associated_token_program
    ] = accounts 
    else {
        return Err(ProgramError::NotEnoughAccountKeys)
    };


    let bump = data[17];
    let seed = [b"fundraiser".as_ref(), maker.address().as_ref(), &[bump]];

    let fundraiser_pda = derive_address(&seed, None, &crate::ID.to_bytes());
    assert_eq!(fundraiser_pda, *fundraiser.address().as_array());

    //let amount_to_raise = unsafe { *(data.as_ptr().add(1) as *const u64) }
    //let current_amount = unsafe { *(data.as_ptr().add(8) as *const u64) }

    let duration = data[16];

    let amount_to_raise = u64::from_le_bytes(data[0..8].try_into().unwrap());
    
    let mint_data = mint_to_raise.try_borrow()?;
    if mint_data.len() < 45 {
        return Err(ProgramError::InvalidAccountData);
    }

    let decimals = mint_data[44];

    let scaled_min = MIN_AMOUNT_TO_RAISE.checked_mul(10u64.pow(decimals as u32)).ok_or(ProgramError::ArithmeticOverflow)?;
    if amount_to_raise < scaled_min {
        return Err(ProgramError::InvalidArgument);
    }

    if fundraiser.owned_by(&crate::ID) {
        return Err(ProgramError::AccountAlreadyInitialized);
    }

    let current_amount = 0;
    let time_started = i64::from_le_bytes(data[8..16].try_into().unwrap());

    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_array()),
        Seed::from(bump_bytes.as_ref()),
    ];
    let signer = Signer::from(&signer_seeds);

    CreateAccount {
        from: maker,
        to: fundraiser,
        lamports: Rent::get()?.try_minimum_balance(Fundraiser::LEN)?,
        space: Fundraiser::LEN as u64,
        owner: &crate::ID
    }
    .invoke_signed(&[signer])?;

    let fundraiser_data = Fundraiser::from_account_info(fundraiser)?;
    fundraiser_data.set_maker(maker.address());
    fundraiser_data.set_mint_to_raise(mint_to_raise.address());
    fundraiser_data.set_amount_to_raise(amount_to_raise);
    fundraiser_data.set_current_amount(current_amount);
    fundraiser_data.set_time_started(time_started);
    fundraiser_data.duration = duration;
    fundraiser_data.bump = bump;

    pinocchio_associated_token_account::instructions::Create {
        funding_account: maker,
        account: vault,
        wallet: fundraiser,
        mint: mint_to_raise,
        token_program,
        system_program
    }
    .invoke()?;

    Ok(())
}