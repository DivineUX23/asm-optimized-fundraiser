use pinocchio::cpi::{Seed, Signer};
use crate::{Account, read_token_amount, state::Fundraiser, validate_ata};

#[inline(always)]
pub fn process_checker_instruction(accounts: &[Account; 9], data: &[u8]) -> Result<(), u32> {
    let maker = accounts[0];
    let mint_to_raise = accounts[1];
    let fundraiser = accounts[2];
    let vault = accounts[3];
    let maker_ata = accounts[4];
    let _system_program = accounts[5];
    let _token_program = accounts[6];


    if !validate_ata(
        maker_ata.data(), 
        mint_to_raise.key() as *const u8, 
        maker.key() as *const u8
    ) {
        return Err(21);
    }

    let bump = unsafe { *( data.as_ptr() ) };

    let fundraiser_data = Fundraiser::from_ptr(fundraiser.data());

    let vault_amount = read_token_amount(vault.data());

    if vault_amount < fundraiser_data.amount_to_raise() {
        return Err(20);
    }


    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(fundraiser_data.maker()),
        Seed::from(bump_bytes.as_ref()),
    ];
    let signer = Signer::from(&signer_seeds);


    let _ = pinocchio_token::instructions::Transfer::new(
        unsafe { &*(&vault as *const Account as *const _) },
        unsafe { &*(&maker_ata as *const Account as *const _) },
        unsafe { &*(&fundraiser as *const Account as *const _) },
        vault_amount
    ).invoke_signed(&[signer]);


    maker.set_lamport(maker.lamports() + fundraiser.lamports());
    fundraiser.set_lamport(0);

    //let _ = fundraiser.close();

    Ok(())
}