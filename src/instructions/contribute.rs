use pinocchio::cpi::{Seed, Signer};
use pinocchio_system::instructions::CreateAccount; 

use crate::{ID_BYTES, Account, asm_ops::{validate_ata},
    constants::{CONTRIBUTOR_RENT_EXEMPT, MAX_CONTRIBUTION_DENOMINATOR, SECONDS_TO_DAYS}, 
    state::{Contributor, Fundraiser}};


#[inline(always)]
pub fn process_contribute_instruction(accounts: &[Account; 10], data: &[u8]) -> Result<(), u32> {
    let contributor = accounts[0];
    let mint_to_raise = accounts[1];
    let fundraiser = accounts[2];
    let contributor_account = accounts[3];
    let contributor_ata = accounts[4];
    let vault = accounts[5];
    let clock_sysvar = accounts[6];
    let _system_program = accounts[7];
    let _token_program = accounts[8];
    let _associated_token_program  = accounts[9];
    

    if !validate_ata(
        contributor_ata.data(), 
        mint_to_raise.key() as *const u8, 
        contributor.key() as *const u8
    ) {
        return Err(21);
    }


    let amount = unsafe { ( data.as_ptr() as *const u64 ).read_unaligned() };
    //let amount = read_token_amount(data.as_ptr() as *const u8);

    let bump = unsafe { *( data.as_ptr().add(8) ) };

    let fundraiser_data = Fundraiser::from_ptr(fundraiser.data());

    let max_contribution = fundraiser_data.amount_to_raise() / MAX_CONTRIBUTION_DENOMINATOR;

    if amount == 0 || amount > max_contribution {
        return Err(20);
    }
    /*
    let current_time = {
        use pinocchio::sysvars::{Sysvar, clock::Clock};
        Clock::get().unwrap().unix_timestamp
    };*/

    let current_time = unsafe {
        let clock_data = clock_sysvar.data();
        ( clock_data.add(32) as *const i64 ).read_unaligned()
    };

    let days = ((current_time - fundraiser_data.time_started())/SECONDS_TO_DAYS) as u8;


    //come back to this bug in your test:

    if fundraiser_data.duration >= days {
        return Err(20);
    }

    /*
    if days >= fundraiser_data.duration {
        return Err(20);
    }
    */



    if !contributor_account.owned_by(&ID_BYTES) {

        let bump_bytes = [bump];
        let signer_seeds = [
            Seed::from(b"contributor"),
            Seed::from(contributor.key()),
            Seed::from(bump_bytes.as_ref()),
        ];
        let signer = Signer::from(&signer_seeds);

        let _ = CreateAccount {
            from: unsafe { &*(&contributor as *const Account as *const _) },
            to: unsafe { &*(&contributor_account as *const Account as *const _) },
            lamports: CONTRIBUTOR_RENT_EXEMPT,
            space: Contributor::LEN as u64,
            owner: &crate::ID
        }
        .invoke_signed(&[signer]);

        Contributor::from_ptr(contributor_account.data()).set_amount(amount);

    } else {

        let contributor_data = Contributor::from_ptr(contributor_account.data());
        
        let fund_amount = contributor_data.amount();

        if fund_amount + amount > max_contribution {
            return Err(20);
        }

        contributor_data.set_amount(fund_amount + amount);

    }

    let _ = pinocchio_token::instructions::Transfer::new(
        unsafe { &*(&contributor_ata as *const Account as *const _) },
        unsafe { &*(&vault as *const Account as *const _) },
        unsafe { &*(&contributor as *const Account as *const _) },
        amount
    ).invoke();


    let fund_amount = fundraiser_data.current_amount();
    fundraiser_data.set_current_amount(fund_amount + amount);

    Ok(())
}