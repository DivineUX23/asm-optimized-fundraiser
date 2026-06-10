#[inline(always)]
pub fn keys_equal(a: *const u8, b: *const u8) -> bool {
    let a0 = unsafe { (a as *const u64).read() };
    let b0 = unsafe { (b as *const u64).read() };
    let a1 = unsafe { (a.add(8) as *const u64).read() };
    let b1 = unsafe { (b.add(8) as *const u64).read() };
    let a2 = unsafe { (a.add(16) as *const u64).read() };
    let b2 = unsafe { (b.add(16) as *const u64).read() };
    let a3 = unsafe { (a.add(24) as *const u64).read() };
    let b3 = unsafe { (b.add(24) as *const u64).read() };
    ((a0 ^ b0) | (a1 ^ b1) | (a2 ^ b2) | (a3 ^ b3)) == 0
}

#[cfg(target_arch = "bpf")]
#[inline(always)]
pub unsafe fn keys_equal_asm(a: *const u8, b: *const u8) -> bool {
    let result: u64;
    unsafe {
        core::arch::asm!(
            "ldxdw {r0}, [{a}+0]",
            "ldxdw {r1}, [{b}+0]",
            "xor {r0}, {r1}",

            "ldxdw {r1}, [{a}+8]",
            "ldxdw {r2}, [{b}+8]",
            "xor {r1}, {r2}",
            "or {r0}, {r1}", 

            "ldxdw {r1}, [{a}+16]",
            "ldxdw {r2}, [{b}+16]",
            "xor {r1}, {r2}",
            "or {r0}, {r1}", 

            "ldxdw {r1}, [{a}+24]",
            "ldxdw {r2}, [{b}+24]",
            "xor {r1}, {r2}",
            "or {r0}, {r1}", 

            a = in(reg) a,
            b = in(reg) b,
            r0 = out(reg) result,
            r1 = out(reg) _,
            r2 = out(reg) _,
            options(pure, readonly, nostack),
        );
    }
    result == 0
}

#[inline(always)]
pub fn copy32(dst: *mut u8, src: *const u8){
    let w0 = unsafe { (src as *const u64).read() };
    let w1 = unsafe { (src.add(8) as *const u64).read() };
    let w2 = unsafe { (src.add(16) as *const u64).read() };
    let w3 = unsafe { (src.add(24) as *const u64).read() };

    unsafe { (dst as *mut u64).write(w0) };
    unsafe { (dst.add(8) as *mut u64).write(w1) };
    unsafe { (dst.add(16) as *mut u64).write(w2) };
    unsafe { (dst.add(24) as *mut u64).write(w3) };
}

#[inline(always)]
pub fn read_u64_at<const OFFSET: usize>(base: *const u8) -> u64 {
    unsafe { (base.add(OFFSET) as *const u64).read() }
}

#[inline(always)]
pub fn write_u64_at<const OFFSET: usize>(base: *mut u8, data: u64) {
    unsafe { (base.add(OFFSET) as *mut u64).write(data) };
}

#[inline(always)]
pub fn read_i64_at<const OFFSET: usize>(base: *const u8) -> i64 {
    unsafe { (base.add(OFFSET) as *const i64).read() }
}

#[inline(always)]
pub fn sub_unchecked(a: u64, b: u64) -> u64 {
    a.wrapping_sub(b)
}

#[inline(always)]
pub fn branchless_max(a: u64, b: u64) -> u64 {
    let mask = (b > a) as u64;

    let mask = mask.wrapping_neg();
    a ^ ((a ^ b) & mask)
}

#[inline(always)]
pub fn validate_ata(
    ata_data: *const u8,
    mint: *const u8,
    owner: *const u8
) -> bool {
    keys_equal(ata_data, mint) & keys_equal(unsafe { ata_data.add(32) }, owner)
}

#[inline(always)]
pub fn read_clock_timestamp(clock_data: *const u8) -> i64 {
    read_i64_at::<32>(clock_data)
}

#[inline(always)]
pub fn read_token_amount(token_account_data: *const u8) -> u64 {
    read_u64_at::<64>(token_account_data)
}

#[inline(always)]
pub fn read_mint_decimals(mint_data: *const u8) -> u8 {
    unsafe { *mint_data.add(44) }
}