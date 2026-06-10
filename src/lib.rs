use pinocchio::address::declare_id;

pub mod asm_ops;
pub use asm_ops::*;

pub mod constants;
mod test;

mod instructions;
mod state;

use instructions::*;

declare_id!("96TFrsG998MvvrfuShRQmSemkzN555pnidGF4gquJsKr");

//pub const ID_BYTES: [u8; 32] = pinocchio_pubkey::pubkey!("96TFrsG998MvvrfuShRQmSemkzN555pnidGF4gquJsKr");

pub const ID_BYTES: [u8; 32] = [
  120,  67,  22,  98, 110, 201, 187, 203, 
   87, 237, 248,  37, 203, 199,  49,  23, 
  134,  97, 209,  76, 143, 200, 135, 105, 
  239, 131,  29, 202, 149,  43, 214, 117
];

const MAX_PERMITTED_DATA_INCREASE: usize = 10_240;
const NON_DUP_MARKER: u8 = 0xff;

#[derive(Clone, Copy)]
pub struct Account(pub *mut u8);

impl Account {

    #[inline(always)]
    pub fn key(self) -> &'static [u8; 32] {
        // Offset 8: Pubkey
        unsafe { &*(self.0.add(8) as *const [u8; 32]) }
    }

    #[inline(always)]
    pub fn owner(self) -> &'static [u8; 32] {
        unsafe { &*(self.0.add(40) as *const [u8; 32]) }
    }

    #[inline(always)]
    pub fn lamports(self) -> u64 {
        unsafe { (self.0.add(72) as *const u64).read() }
    }

    #[inline(always)]
    pub fn set_lamport(self, lamports: u64) {
        unsafe { (self.0.add(72) as *mut u64).write(lamports); }
    }

    #[inline(always)]
    pub fn data_len(self) -> usize {
        unsafe { (self.0.add(80) as *const u64).read() as usize }
    }

    #[inline(always)]
    pub fn data(self) -> *mut u8{
        unsafe { self.0.add(88) }
    }

    #[inline(always)]
    pub fn read_u64(self, data_offset: usize) -> u64 {
        unsafe { (self.0.add(88 + data_offset) as *const u64).read() }
    }

    #[inline(always)]
    pub fn write_u64(self, data_offset: usize, data: u64) {
        unsafe { (self.0.add(88 + data_offset) as *mut u64).write(data) }
    }

    #[inline(always)]
    pub fn read_u8(self, data_offset: usize) -> u8 {
        unsafe { *self.0.add(88 + data_offset) }
    }

    #[inline(always)]
    pub fn write_u8(self, data_offset: usize, data: u8) {
        unsafe { *self.0.add(88 + data_offset) = data; }
    }

    #[inline(always)]
    pub fn owned_by(self, program_id: *const [u8; 32]) -> bool {
        unsafe { keys_equal(self.0.add(40), program_id as *const u8) }
    }

    #[inline(always)]
    pub fn is_signer(self) -> u8 {
        unsafe { *self.0.add(1) }
    }

    #[inline(always)]
    pub fn is_writable(self) -> u8 {
        unsafe { *self.0.add(2) }
    }

}

#[inline(always)]
unsafe fn parse_input(
    input: *mut u8,
    out: &mut [Account; 10],
) -> (&'static [u8], *const [u8; 32]) {
    let num_accounts = unsafe { *(input as *const u64) } as usize;
    let mut ptr = unsafe { input.add(8) }; // shift point to after discriminator



    let count = num_accounts.min(9);
    let mut i = 0;

    while i < num_accounts {  // walk ALL accounts, not just count
        let dup = unsafe { *ptr };
        if dup == NON_DUP_MARKER {
            let data_len = unsafe { (ptr.add(80) as *const u64).read() } as usize;

            if i < count {
                out[i] = Account(ptr);
            }

            ptr = unsafe { ptr.add(88 + data_len + MAX_PERMITTED_DATA_INCREASE + 8) };
            ptr = ((ptr as usize + 7) & !7) as *mut u8;

        } else {
            if i < count {
                out[i] = out[dup as usize];
            }
            ptr = unsafe { ptr.add(8) };
        }

        i += 1;
    }


    let ix_len = unsafe { *(ptr as *const u64) } as usize; // reading the length of datas

    ptr = unsafe { ptr.add(8) };
    let ix_data = unsafe { core::slice::from_raw_parts(ptr as *const u8, ix_len) };
    let program_id = unsafe { ptr.add(ix_len) } as *const [u8; 32];

    (ix_data, program_id)
}


//#[cfg(target_os = "solana")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn entrypoint(input: *mut u8) -> u64 {
    let mut accounts  = [Account(core::ptr::null_mut()); 10];

    //let accounts = pinocchio::account_info::AccountInfo::from_raw(input);


    let (ix_data, _program_id) = unsafe { parse_input(input, &mut accounts) };

    if ix_data.is_empty() {
        return 4;
    }

    let discriminator = unsafe { *ix_data.as_ptr() };
    let data = unsafe { core::slice::from_raw_parts(ix_data.as_ptr().add(1), ix_data.len() -1) };


    let result = match discriminator {        
        0 => process_initialize_instruction(&accounts, data).map_err(|e| e as u64),
        1 => process_contribute_instruction(&accounts, data).map_err(|e| e as u64),
        2 => process_checker_instruction(&accounts, data).map_err(|e| e as u64),
        3 => process_refund_instruction(&accounts, data).map_err(|e| e as u64),
        _ => Err(4),
    };

    match result {
        Ok(()) => 0,
        Err(e) => e,
    }

}