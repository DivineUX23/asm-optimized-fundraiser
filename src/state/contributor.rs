use crate::asm_ops::{read_u64_at, write_u64_at};

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Contributor {
    amount: [u8; 8]
}

impl Contributor {
    pub const LEN: usize = 8;

    #[inline(always)]
    pub fn from_ptr(data: *mut u8) -> &'static mut Self {
        unsafe { &mut *(data as *mut Self) }
    }

    #[inline(always)]
    pub fn from_ptr_checked(data: *mut u8, len: usize) -> Result<&'static mut Self, u32> {

        if len != Self::LEN {
            return Err(0x2);
        }
        Ok(Self::from_ptr(data))
    }

    #[inline(always)]
    pub fn amount(&self) -> u64 {
        read_u64_at::<0>(self.amount.as_ptr())
    }

    #[inline(always)]
    pub fn set_amount(&mut self, amount: u64) {
        write_u64_at::<0>(self.amount.as_mut_ptr(), amount);
    }
}