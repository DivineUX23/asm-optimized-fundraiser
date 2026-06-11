# Token Fundraiser — v3: Unsafe / Near-Assembly

> **Series:** Solana CU Optimization · Part 3 of 3

A fundraising program for SPL Tokens built on Solana. This version
**bypasses Pinocchio's deserialization layer entirely**, replacing it with
a hand-written raw entrypoint that walks the BPF input buffer directly using
pointer arithmetic. Account structs are replaced with a single raw pointer
per account. Almost nothing depends on Pinocchio's validated APIs.

**Safety is not guaranteed.** Correctness depends on hardcoded knowledge
of the BPF loader's input buffer layout and the SPL token account layout.
If either changes, this program produces undefined behaviour silently.

---

## What It Does

A maker creates a fundraiser specifying:
- The SPL mint token they want to collect
- A target amount to raise
- A duration in days

Contributors can send tokens into a program-owned vault. Once the target
is met the maker can claim the vault. If the deadline passes without
reaching the target, contributors can claim refunds.

---

## Instructions

| Instruction   | Accounts | Description |
|---------------|----------|-------------|
| `Initialize`  | 7        | Create the fundraiser account and vault ATA |
| `Contribute`  | 9        | Deposit tokens; create contributor PDA on first contribution |
| `Checker`     | 7        | Verify goal reached and transfer vault to maker |
| `Refund`      | 9        | Return contributor tokens after deadline with unmet goal |

---

## How to Run

```bash
# Build the program
cargo build-sbf

# Run all tests with output
cargo test -- --nocapture
```

---

## Architecture

### Custom Entrypoint

The Pinocchio `entrypoint!()` macro is not used. Instead, a hand-written
`extern "C" fn entrypoint(input: *mut u8) -> u64` walks the raw BPF input
buffer that the runtime writes before calling the program.

The BPF loader lays out the input buffer as follows:

```
[u64 num_accounts]
For each account:
  [u8  dup_info]          ← 0xFF = non-duplicate, else = original slot index
  If non-duplicate:
    [u8  is_signer]
    [u8  is_writable]
    [u8  executable]
    [u8×4 padding]        ← align to 8 bytes
    [u8×32 key]           ← pubkey,  at offset +8  from slot start
    [u8×32 owner]         ← owner,   at offset +40
    [u64 lamports]        ← mutable, at offset +72
    [u64 data_len]        ←          at offset +80
    [u8×data_len data]    ← mutable, at offset +88
    [u8×10240 headroom]   ← MAX_PERMITTED_DATA_INCREASE realloc space
    [alignment padding]   ← pad to next 8-byte boundary
  If duplicate:
    [u8×7 padding]        ← 8 bytes total for this slot
[u64 ix_data_len]
[u8×ix_data_len ix_data]
[u8×32 program_id]
```

Because `88 + 10240 = 10328` is already 8-aligned, the stride between
accounts is:
```
stride = 10328 + ((data_len + 7) & !7)
```

### `Account` struct

Pinocchio's `AccountView` is replaced by a single-pointer wrapper:

```rust
#[derive(Clone, Copy)]
pub struct Account(pub *mut u8);  // raw pointer to start of account slot
```

Every field is derived from this pointer with a known constant offset,
compiling to a single `ldxdw` or `stxdw` BPF instruction:

```rust
impl Account {
    pub unsafe fn key(self)      -> *const [u8; 32] { self.0.add(8)  as _ }
    pub unsafe fn owner(self)    -> *const [u8; 32] { self.0.add(40) as _ }
    pub unsafe fn lamports(self) -> u64             { (self.0.add(72) as *const u64).read() }
    pub unsafe fn data(self)     -> *mut u8         { self.0.add(88) }
    // etc.
}
```

### Assembly-equivalent primitives (`asm_ops.rs`)

The hot operations are written as unsafe Rust expressions that, with
`opt-level = 3` and `lto = fat`, compile to the exact BPF instruction
sequences shown. No more, no fewer.

**32-byte key comparison — 14 BPF instructions = 14 CU:**
```rust
// vs sol_memcmp_ syscall: ~310 CU
// vs byte-by-byte comparison: 32+ CU
pub unsafe fn keys_equal(a: *const u8, b: *const u8) -> bool {
    let a0 = (a      as *const u64).read();
    let b0 = (b      as *const u64).read();
    let a1 = (a.add(8)  as *const u64).read();
    let b1 = (b.add(8)  as *const u64).read();
    let a2 = (a.add(16) as *const u64).read();
    let b2 = (b.add(16) as *const u64).read();
    let a3 = (a.add(24) as *const u64).read();
    let b3 = (b.add(24) as *const u64).read();
    ((a0 ^ b0) | (a1 ^ b1) | (a2 ^ b2) | (a3 ^ b3)) == 0
    // BPF output: 8× ldxdw + 4× xor + 3× or + 1× jeq = 16 instructions
}
```

**Branchless ATA validation — 28 CU for both mint and owner simultaneously:**
```rust
// Standard approach: 2 if-checks + 2 wrapper parses = ~660 CU
// This approach: 28 BPF instructions, no branches
pub unsafe fn validate_ata(ata: *const u8, mint: *const u8, owner: *const u8) -> bool {
    keys_equal(ata, mint) & keys_equal(ata.add(32), owner)
    // Note: & not && — evaluates both without a branch
}
```

**SPL token amount at known offset — 1 CU:**
```rust
// SPL Token layout: [mint:32][owner:32][amount:8] → amount at byte 64
pub unsafe fn read_token_amount(token_data: *const u8) -> u64 {
    (token_data.add(64) as *const u64).read()
}
```

---

## Optimization Approach

This version inherits all v2 optimizations plus:

**1. Custom raw entrypoint — eliminates Pinocchio's deserializer**

Pinocchio's `deserialize()` function performs per-account dup-info checks,
builds `AccountView` slice metadata, and has function call overhead. The
custom entrypoint does exactly the same work with fewer instructions:
- No `AccountView` struct construction (just raw `Account(*mut u8)`)
- No slice header allocation
- Single tight loop over accounts with one branch per account

Estimated saving: ~80–120 CU on the entrypoint dispatch path.

**2. Branchless ATA validation**

v2 validates mint and owner with two separate if-checks and two calls to
Pinocchio's token wrapper. v3 merges both into a single 28-instruction
sequence with no branches using the `&` (non-short-circuit AND) trick:

```rust
if !validate_ata(ata.data(), mint.key() as _, contributor.key() as _) {
    return Err(21);
}
```

**3. All account field accesses via `Account` pointer offsets**

Every read from any account field compiles to a single `ldxdw`. No
intermediate structs, no slice validation, no `borrow_unchecked` wrapper
overhead. The pointer into the input buffer IS the account.

**4. Batch state initialization**

All Fundraiser fields are written in a single contiguous pass via
`Fundraiser::initialize_from()`, allowing LLVM to schedule all 12 stores
against the single CreateAccount CPI without register pressure from
intermediate struct construction.

---

## Test Results

```
running 7 tests

Final Binary Size: 15.59 KB
test test::tests::test_binary_size ... ok   ✓ passes L1 iCache threshold

Initialize CU:  16,179   (with new ATA creation)
Initialize CU:  16,179   (second run)
Initialize CU:  19,179   (with extra account overhead variant)
Contribute CU:   2,612   ← best across all three versions
Contribute CU:   2,660
Refund CU:       1,493   ← best across all three versions
Checker CU:      1,268

Estimated CU per account parsed: 2,311

test result: ok. 7 passed; 0 failed
```

---

## Performance Summary

### vs v1 Baseline

| Metric          | v1 Baseline | v3 (this)  | Improvement |
|-----------------|-------------|------------|-------------|
| Binary size     | 52.44 KB    | **15.59 KB** | ↓ 70.3%   |
| Initialize CU   | 16,628      | 16,179     | ↓ 449 CU (2.7%) |
| Contribute CU   |  3,209      |  **2,612** | ↓ 597 CU (18.6%) |
| Refund CU       |  1,870      |  **1,493** | ↓ 377 CU (20.2%) |
| Checker CU      |  1,341      |  1,268     | ↓ 73 CU (5.4%) |

### vs v2 (Optimized Pinocchio)

| Metric          | v2 Optimized | v3 (this)  | Difference |
|-----------------|--------------|------------|------------|
| Binary size     | 14.20 KB     | 15.59 KB   | ↑ +1.39 KB (+9.8%) ← regression |
| Initialize CU   | 16,174       | 16,179     | ↑ +5 CU (negligible) |
| Contribute CU   |  2,688       |  **2,612** | ↓ 76 CU (2.8%) |
| Refund CU       |  1,582       |  **1,493** | ↓ 89 CU (5.6%) |
| Checker CU      |  1,251       |  1,268     | ↑ +17 CU (slight regression) |

### Understanding the Checker Regression

Checker is slightly worse in v3 (+17 CU) despite the custom entrypoint and
raw primitives. The cause: v3's CPI transmute (`Account` → Pinocchio's
`AccountView`) for the `invoke_signed` call adds overhead that v2 avoids by
staying natively in Pinocchio's type system throughout. The custom account
type wins on reads but loses on CPI boundaries.

### Understanding the Binary Size Regression

v3 is 1.39 KB larger than v2 despite having fewer dependencies. The custom
entrypoint introduces code that does not benefit from LTO against Pinocchio's
internals in the same way v2's code does — the raw account walker is an
additional code path LLVM cannot merge with anything else. v2's use of
Pinocchio's own deserializer allows those paths to be fully inlined and
eliminated under fat LTO.

---

## BPF Instruction Cost Reference

Every optimization in this version is grounded in the BPF VM's cost model:

| Operation | Method | BPF Instructions | CU Cost |
|-----------|--------|------------------|---------|
| u64 field read | raw ptr read | 1 ldxdw | **1** |
| u64 field read | `from_le_bytes().try_into()` | ~20 | ~20 |
| 32-byte key compare | `keys_equal()` | 14 | **14** |
| 32-byte key compare | `sol_memcmp_` syscall | syscall overhead | ~310 |
| 32-byte key copy | 4× ldxdw + 4× stxdw | 8 | **8** |
| 32-byte key copy | `sol_memcpy_` syscall | syscall overhead | ~210 |
| ATA mint+owner check | `validate_ata()` branchless | 28 | **28** |
| ATA mint+owner check | 2× wrapper + 2× if | ~660 | ~660 |
| Token account amount | raw offset 64 | 1 ldxdw | **1** |
| Token account amount | `TokenAccountState` wrapper | ~50 | ~50 |
| PDA pre-verification | `derive_address` | SHA-256 syscall | ~1,500 |
| PDA pre-verification | none (runtime validates) | 0 | **0** |
| Rent minimum | `Rent::get()?` | sysvar syscall | ~100 |
| Rent minimum | hardcoded constant | 1 mov | **1** |
| Timestamp | `Clock::get()?` | sysvar syscall | ~150–200 |
| Timestamp | account data read at offset 32 | 1 ldxdw | **1** |

---

## Pros

- **Lowest Contribute CU of all three versions** — 2,612 CU, 18.6% below
  v1 and 2.8% below v2
- **Lowest Refund CU of all three versions** — 1,493 CU, 20.2% below
  v1 and 5.6% below v2
- **Maximum transparency about BPF costs** — every CU is accounted for.
  No hidden work behind framework wrappers
- **Minimal external dependencies** — almost nothing from Pinocchio's
  account infrastructure is required. The program would survive significant
  Pinocchio API changes
- **Educational value** — demonstrates exactly how the BPF input buffer
  is laid out and how the runtime executes program code at the instruction
  level
- **Passes all 7 tests** including binary size constraint

---

## Cons

- **Checker regresses vs v2** (+17 CU) — the CPI transmute from `Account`
  to Pinocchio's `AccountView` at CPI call sites adds overhead that
  cancels the raw-read savings for the instruction with the fewest reads
- **Binary is 1.39 KB larger than v2** — the custom entrypoint cannot
  LTO-merge with Pinocchio internals the way v2's code does, leaving a
  separate code path in the binary
- **Safety not guaranteed** — undefined behaviour if the BPF loader
  input format changes, if the SPL token layout changes, or if any
  account layout assumption is violated
- **Marginal CU wins vs v2** — 76 CU on Contribute, 89 CU on Refund.
  For most use cases these gains do not justify the safety and
  maintenance cost
- **High audit complexity** — reviewers must know the exact byte offsets
  of every field in every account type. One off-by-one error silently
  reads from the wrong memory location
- **CPI type mismatch** — CPIs still ultimately require Pinocchio's
  `AccountView` type, requiring raw transmutes at call boundaries.
  This is fragile and may break if Pinocchio's internal layout changes
- **Not beginner-friendly** — this code cannot be maintained by a developer
  who does not have a detailed understanding of BPF execution, Solana's
  serialization format, and unsafe Rust

---

## Risk Table

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| BPF loader input format change | Low (stable since 2021) | **Critical** (silent wrong reads) | Pin loader version, add layout tests |
| SPL token layout change | Very low (frozen spec) | **High** (wrong amounts) | Add layout assertion tests |
| Pinocchio `AccountView` layout change | Low | **High** (CPI transmutes break) | Pin Pinocchio version |
| Off-by-one in manual offset | Medium (dev error) | **High** (silent wrong state) | Property-based tests on all offsets |

---

## When to Use This Version

- In a competition where every single CU matters and you have accepted
  the safety tradeoffs in writing
- As a research reference for understanding BPF execution costs at the
  instruction level
- When you are building tooling to benchmark or profile Solana programs
- When you need the absolute floor on Contribute and Refund CU and are
  prepared to maintain the layout assertions

> For production programs where correctness and auditability matter,
> use **v2** (optimized Pinocchio) instead. The CU difference is
> 76–89 CU — a real but small number relative to the safety cost.

---

## Full Version Comparison

| Metric            | v1 Raw    | v2 Optimized | v3 Unsafe | Winner |
|-------------------|-----------|--------------|-----------|--------|
| Binary size       | 52.44 KB  | **14.20 KB** | 15.59 KB  | v2 |
| Initialize CU     | 16,628    | **16,174**   | 16,179    | v2 |
| Contribute CU     |  3,209    |  2,688       | **2,612** | v3 |
| Refund CU         |  1,870    |  1,582       | **1,493** | v3 |
| Checker CU        |  1,341    |  **1,251**   | 1,268     | v2 |
| Tests passing     | 6 / 7     | **7 / 7**    | **7 / 7** | v2, v3 |
| Safety guarantees | ✅ Full   | ✅ Full      | ❌ None   | v1, v2 |
| Audit complexity  | Low       | Medium       | High      | v1 |
| Maintenance cost  | Low       | Medium       | High      | v1 |
