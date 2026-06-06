use pinocchio::{
    AccountView, ProgramResult, cpi::{Seed, Signer}, error::ProgramError
};
use crate::state::{Fundraiser};

pub fn process_checker_instruction(accounts: &mut [AccountView], data: &[u8]) -> ProgramResult {
    let [
        maker,
        mint_to_raise,
        fundraiser,
        vault,
        maker_ata,
        _system_program,
        _token_program,
    ] = accounts 
    else {
        return Err(ProgramError::NotEnoughAccountKeys)
    };

    {
        let maker_ata_state = pinocchio_token::state::Account::from_account_view(maker_ata)?;
        if maker_ata_state.owner() != maker.address() {
            return Err(ProgramError::IllegalOwner);
        }
        if maker_ata_state.mint() != mint_to_raise.address() {
            return Err(ProgramError::InvalidAccountData);
        }
    }

    let fundraiser_data = Fundraiser::from_account_info(fundraiser)?;

    let vault_data = unsafe {vault.borrow_unchecked()};
    let vault_amount = u64::from_le_bytes(vault_data[64..72].try_into().unwrap());

    if vault_amount < fundraiser_data.amount_to_raise() {
        return Err(ProgramError::InvalidArgument);
    }

    let bump = data[0];

    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.address().as_array()),
        Seed::from(bump_bytes.as_ref()),
    ];
    let signer = Signer::from(&signer_seeds);


    pinocchio_token::instructions::Transfer::new(vault, maker_ata, fundraiser, vault_amount)
        .invoke_signed(&[signer])?;

    let fundraiser_lamports = fundraiser.lamports();
    maker.set_lamports(maker.lamports() + fundraiser_lamports);
    fundraiser.set_lamports(0);

    let _ = fundraiser.close();

    Ok(())
}