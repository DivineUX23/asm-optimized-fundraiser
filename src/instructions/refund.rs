use pinocchio::cpi::{Seed, Signer};
use crate::{Account, asm_ops::{read_token_amount, validate_ata},
    constants::SECONDS_TO_DAYS,
    state::{Contributor, Fundraiser}};

#[inline(always)]
pub fn process_refund_instruction(accounts: &[Account; 10], data: &[u8]) -> Result<(), u32> {

    let contributor = accounts[0];
    let maker = accounts[1];
    let mint_to_raise = accounts[2];
    let fundraiser = accounts[3];
    let contributor_account = accounts[4];
    let contributor_ata = accounts[5];
    let vault = accounts[6];
    let clock_sysvar = accounts[7];
    let _system_program = accounts[8];
    let _token_program = accounts[9];
    

    if !validate_ata(
        contributor_ata.data(), 
        mint_to_raise.key() as *const u8, 
        contributor.key() as *const u8
    ) {
        return Err(21);
    }

    let bump = unsafe { *( data.as_ptr() ) };

    let fundraiser_data = Fundraiser::from_ptr(fundraiser.data());


    let contributor_data = Contributor::from_ptr(contributor_account.data());

    /*
    let current_time = {
        use pinocchio::sysvars::{Sysvar, clock::Clock};
        Clock::get().unwrap().unix_timestamp
    };
    */
    let current_time = {
        let clock_data = clock_sysvar.data();
        unsafe { ( clock_data.add(32) as *const i64 ).read_unaligned() }
    };

    if fundraiser_data.duration < ((current_time - fundraiser_data.time_started())/SECONDS_TO_DAYS) as u8 {
        return Err(20);
    }

    let vault_amount = read_token_amount(vault.data());

    if vault_amount >= fundraiser_data.amount_to_raise() {
        return Err(20);
    }

    fundraiser_data.set_current_amount(fundraiser_data.current_amount() - contributor_data.amount());


    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.key()),
        Seed::from(bump_bytes.as_ref()),
    ];
    let signer = Signer::from(&signer_seeds);


    let _ = pinocchio_token::instructions::Transfer::new(
        unsafe { &*(&vault as *const Account as *const _) }, 
        unsafe { &*(&contributor_ata as *const Account as *const _) }, 
        unsafe { &*(&fundraiser as *const Account as *const _) },
        contributor_data.amount()
    )
        .invoke_signed(&[signer]);


    contributor.set_lamport(contributor.lamports() + contributor_account.lamports());
    contributor_account.set_lamport(0);

    //let _ = contributor_account.close();

    Ok(())
}