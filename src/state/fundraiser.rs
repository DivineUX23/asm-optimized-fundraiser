use crate::asm_ops::{copy32, read_u64_at, read_i64_at, write_u64_at};

#[repr(C)]
pub struct Fundraiser {
    maker: [u8; 32],
    mint_to_raise: [u8; 32],
    amount_to_raise: [u8; 8],
    current_amount: [u8; 8],
    time_started: [u8; 8],
    pub duration: u8,
    pub bump: u8,
}

//const _: () = assert!(core::mem::size_of::<Fundraiser>() == 90);

impl Fundraiser {
    pub const LEN: usize = 90;

    pub const MAKER_OFFSET: usize = 0;
    pub const MINT_OFFSET: usize = 32;
    pub const AMOUNT_TO_RAISE_OFFSET: usize = 64;
    pub const CURRENT_AMOUNT_OFFSET: usize = 72;
    pub const TIME_STARTED_OFFSET: usize = 80;
    pub const DURATION_OFFSET: usize = 88;
    pub const BUMP_OFFSET: usize = 89;

    /*
    #[inline(always)]
    pub unsafe fn from_account_info_unchecked(account: &mut AccountView) -> &mut Self {
        &mut *(account.borrow_unchecked_mut().as_mut_ptr() as *mut Self)
    }
    

    #[inline(always)]
    pub fn from_account_info(account_info: &mut AccountView) -> Result<&mut Self, ProgramError> {
        let data = unsafe { account_info.borrow_unchecked_mut() };
        if data.len() != Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(unsafe { &mut *(data.as_mut_ptr() as *mut Self) })
    }
    */

    #[inline(always)]
    pub fn from_ptr(data: *mut u8) -> &'static mut Self {
        unsafe { &mut *(data as *mut Self) }
    }

    #[inline(always)]
    pub unsafe fn from_ptr_checked(data: *mut u8, len: usize) -> Result<&'static mut Self, u32> {
        if len != Self::LEN {
            return Err(0x2);
        }
        Ok(Self::from_ptr(data))
    }


    #[inline(always)]
    pub fn maker(&self) -> &[u8; 32] {
        unsafe { &*(self.maker.as_ptr() as *const [u8; 32]) }
    }


    #[inline(always)]
    pub fn set_maker(&mut self, maker: *const u8) {
            copy32(self.maker.as_mut_ptr(), maker);
    }

    #[inline(always)]
    pub fn mint_to_raise(&self) -> &[u8; 32] {
        unsafe { &*(self.mint_to_raise.as_ptr() as *const [u8; 32]) }
    }

    #[inline(always)]
    pub fn set_mint_to_raise(&mut self, mint: *const u8) {
        copy32(self.mint_to_raise.as_mut_ptr(), mint);
    }


    #[inline(always)]
    pub fn amount_to_raise(&self) -> u64 {
        read_u64_at::<0>(self.amount_to_raise.as_ptr())
    }

    #[inline(always)]
    pub fn set_amount_to_raise(&mut self, amount: u64) {
        write_u64_at::<0>(self.amount_to_raise.as_mut_ptr(), amount);
    }

    #[inline(always)]
    pub fn current_amount(&self) -> u64 {
        read_u64_at::<0>(self.current_amount.as_ptr())
    }

    #[inline(always)]
    pub fn set_current_amount(&mut self, amount: u64) {
        write_u64_at::<0>(self.current_amount.as_mut_ptr(), amount);
    }

    #[inline(always)]
    pub fn time_started(&self) -> i64 {
        read_i64_at::<0>(self.time_started.as_ptr())
    }

    #[inline(always)]
    pub fn set_time_started(&mut self, time: i64) {
        unsafe { ( self.time_started.as_mut_ptr() as *mut i64 ).write(time) }
    }

    #[inline(always)]
    pub fn initialize_from(
        dst: *mut u8,
        maker: *const u8,
        mint_to_raise: *const u8,
        amount_to_raise: u64,
        time_started: i64,
        duration: u8,
        bump: u8,
    ) {
        copy32(dst, maker);
        copy32(unsafe { dst.add(32) }, mint_to_raise);
        //write_u64_at::<0>(dst.add(64), amount_to_raise);
        unsafe { (dst.add(64) as *mut u64).write(amount_to_raise) };
        unsafe { (dst.add(72) as *mut u64).write(0u64) };
        unsafe { (dst.add(80) as *mut i64).write(time_started) };
        unsafe { *dst.add(88) = duration };
        unsafe { *dst.add(89) = bump };
    }

}