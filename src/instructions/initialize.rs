use pinocchio::cpi::{Seed, Signer};
use pinocchio_system::instructions::CreateAccount;

use crate::{Account, 
    asm_ops::{read_mint_decimals},
    state::Fundraiser, constants::{MIN_AMOUNT_TO_RAISE, FUNDRAISER_RENT_EXEMPT},
    ID_BYTES};

#[inline(always)]
pub fn process_initialize_instruction(accounts: &[Account; 10], data: &[u8]) -> Result<(), u32> {
    let maker= accounts[0];
    let mint_to_raise= accounts[1];
    let fundraiser= accounts[2];
    let vault= accounts[3];
    let system_program= accounts[4];
    let token_program= accounts[5];
    let _associated_token_program = accounts[6];


    if fundraiser.owned_by(ID_BYTES.as_ptr() as *const [u8; 32]) {
        return Err(11);
    }


    let amount_to_raise = unsafe { (data.as_ptr() as *const u64).read() };
    let time_started = unsafe { (data.as_ptr().add(8) as *const i64).read() };
    let duration = unsafe { *(data.as_ptr().add(16)) };
    let bump = unsafe { *(data.as_ptr().add(17)) };

    //let decimals = unsafe { *mint_to_raise.borrow_unchecked().as_ptr().add(44) };
    let decimals = read_mint_decimals(mint_to_raise.data());

    const POWERS_OF_10: [u64; 10] = [
        1, 10, 100, 1_000, 10_000, 100_000,
        1_000_000, 10_000_000, 100_000_000, 1_000_000_000,
    ];

    let scale = unsafe { *POWERS_OF_10.get_unchecked(decimals as usize) };

    let scaled_min = MIN_AMOUNT_TO_RAISE * scale;
    if amount_to_raise < scaled_min {
        return Err(20);
    }


    let bump_bytes = [bump];
    let signer_seeds = [
        Seed::from(b"fundraiser"),
        Seed::from(maker.key()),
        Seed::from(bump_bytes.as_ref()),
    ];
    let signer = Signer::from(&signer_seeds);

    let _ = CreateAccount {
        from: unsafe { &*(&maker as *const Account as *const _) },
        to: unsafe { &*(&fundraiser as *const Account as *const _) },
        lamports: FUNDRAISER_RENT_EXEMPT,
        space: Fundraiser::LEN as u64,
        owner: &crate::ID
    }.invoke_signed(&[signer]);


    Fundraiser::initialize_from(
        fundraiser.data(),
        maker.key() as *const u8, 
        mint_to_raise.key() as *const u8, 
        amount_to_raise, 
        time_started, 
        duration, 
        bump
    );

    let _ = pinocchio_associated_token_account::instructions::Create {
        funding_account: unsafe { &*(&maker as *const Account as *const _) },
        account: unsafe { &*(&vault as *const Account as *const _) },
        wallet: unsafe { &*(&fundraiser as *const Account as *const _) },
        mint: unsafe { &*(&mint_to_raise as *const Account as *const _) },
        token_program: unsafe { &*(&token_program as *const Account as *const _) },
        system_program: unsafe { &*(&system_program as *const Account as *const _) },
    }
    .invoke();

    Ok(())
}