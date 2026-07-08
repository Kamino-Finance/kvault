//! Anchor-compatible instruction discriminators for Kvault.
//!
//! Each instruction is identified by the first 8 bytes of
//! `SHA-256("global:<snake_case_name>")`. The constants are computed at
//! compile time via a const-compatible SHA-256 implementation.

use sha2::{Digest, Sha256};

/// Compute the 8-byte Anchor discriminator for a given instruction name.
///
/// Uses the convention: `sha256("global:<snake_case_name>")[..8]`.
pub fn compute_discriminator(name: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(format!("global:{name}"));
    let hash = hasher.finalize();
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&hash[..8]);
    disc
}

macro_rules! disc {
    ($(#[$meta:meta])* $name:ident, $ix:expr) => {
        $(#[$meta])*
        pub static $name: [u8; 8] = {
            const PREIMAGE: &[u8] = concat!("global:", $ix).as_bytes();
            sha256_first8(PREIMAGE)
        };
    };
}

const SHA256_K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// Compress a single 64-byte block into the state, returning the new state.
const fn compress_block(h: [u32; 8], block: [u8; 64]) -> [u32; 8] {
    let mut w = [0u32; 64];
    let mut i = 0;
    while i < 16 {
        w[i] = ((block[i * 4] as u32) << 24)
            | ((block[i * 4 + 1] as u32) << 16)
            | ((block[i * 4 + 2] as u32) << 8)
            | (block[i * 4 + 3] as u32);
        i += 1;
    }
    i = 16;
    while i < 64 {
        let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
        let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
        w[i] = w[i - 16]
            .wrapping_add(s0)
            .wrapping_add(w[i - 7])
            .wrapping_add(s1);
        i += 1;
    }

    let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
        (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);

    i = 0;
    while i < 64 {
        let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
        let ch = (e & f) ^ ((!e) & g);
        let temp1 = hh
            .wrapping_add(s1)
            .wrapping_add(ch)
            .wrapping_add(SHA256_K[i])
            .wrapping_add(w[i]);
        let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
        let maj = (a & b) ^ (a & c) ^ (b & c);
        let temp2 = s0.wrapping_add(maj);

        hh = g;
        g = f;
        f = e;
        e = d.wrapping_add(temp1);
        d = c;
        c = b;
        b = a;
        a = temp1.wrapping_add(temp2);
        i += 1;
    }

    [
        h[0].wrapping_add(a),
        h[1].wrapping_add(b),
        h[2].wrapping_add(c),
        h[3].wrapping_add(d),
        h[4].wrapping_add(e),
        h[5].wrapping_add(f),
        h[6].wrapping_add(g),
        h[7].wrapping_add(hh),
    ]
}

/// Const-compatible SHA-256 returning only the first 8 bytes.
/// Supports messages up to 119 bytes (two 64-byte blocks).
pub const fn sha256_first8(msg: &[u8]) -> [u8; 8] {
    let h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];

    let bit_len = (msg.len() as u64) * 8;
    let padded_len = if msg.len() + 9 <= 64 { 64 } else { 128 };

    let mut padded = [0u8; 128];
    let mut i = 0;
    while i < msg.len() {
        padded[i] = msg[i];
        i += 1;
    }
    padded[msg.len()] = 0x80;
    padded[padded_len - 8] = (bit_len >> 56) as u8;
    padded[padded_len - 7] = (bit_len >> 48) as u8;
    padded[padded_len - 6] = (bit_len >> 40) as u8;
    padded[padded_len - 5] = (bit_len >> 32) as u8;
    padded[padded_len - 4] = (bit_len >> 24) as u8;
    padded[padded_len - 3] = (bit_len >> 16) as u8;
    padded[padded_len - 2] = (bit_len >> 8) as u8;
    padded[padded_len - 1] = bit_len as u8;

    // First block
    let mut block0 = [0u8; 64];
    i = 0;
    while i < 64 {
        block0[i] = padded[i];
        i += 1;
    }
    let h = compress_block(h, block0);

    // Second block (if needed)
    let h = if padded_len > 64 {
        let mut block1 = [0u8; 64];
        i = 0;
        while i < 64 {
            block1[i] = padded[64 + i];
            i += 1;
        }
        compress_block(h, block1)
    } else {
        h
    };

    [
        (h[0] >> 24) as u8,
        (h[0] >> 16) as u8,
        (h[0] >> 8) as u8,
        h[0] as u8,
        (h[1] >> 24) as u8,
        (h[1] >> 16) as u8,
        (h[1] >> 8) as u8,
        h[1] as u8,
    ]
}

disc!(
    /// Discriminator for the `deposit` instruction.
    DEPOSIT,
    "deposit"
);
disc!(
    /// Discriminator for the `deposit_with_min_shares_out` instruction.
    DEPOSIT_WITH_MIN_SHARES_OUT,
    "deposit_with_min_shares_out"
);
disc!(
    /// Discriminator for the `buy` instruction.
    BUY,
    "buy"
);
disc!(
    /// Discriminator for the `buy_with_min_shares_out` instruction.
    BUY_WITH_MIN_SHARES_OUT,
    "buy_with_min_shares_out"
);

disc!(
    /// Discriminator for the `withdraw` instruction.
    WITHDRAW,
    "withdraw"
);
disc!(
    /// Discriminator for the `sell` instruction.
    SELL,
    "sell"
);
disc!(
    /// Discriminator for the `withdraw_from_available` instruction.
    WITHDRAW_FROM_AVAILABLE,
    "withdraw_from_available"
);

disc!(
    /// Discriminator for the `invest` instruction.
    INVEST,
    "invest"
);
disc!(
    /// Discriminator for the `invest_with_max_amount` instruction.
    INVEST_WITH_MAX_AMOUNT,
    "invest_with_max_amount"
);

disc!(
    /// Discriminator for the `redeem_in_kind` instruction.
    REDEEM_IN_KIND,
    "redeem_in_kind"
);

/// Known Kvault instruction types, identified by their 8-byte discriminator.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum KvaultInstruction {
    /// Deposit tokens into a vault and receive shares.
    Deposit,
    /// Deposit tokens into a vault with a minimum shares-out guard.
    DepositWithMinSharesOut,
    /// Buy vault shares (same accounts as deposit, different discriminator).
    Buy,
    /// Buy vault shares with a minimum shares-out guard.
    BuyWithMinSharesOut,
    /// Withdraw from available and invested liquidity via a reserve.
    Withdraw,
    /// Sell vault shares (same accounts as withdraw, different discriminator).
    Sell,
    /// Withdraw exclusively from the vault's available (uninvested) balance.
    WithdrawFromAvailable,
    /// Deploy available vault liquidity into a Klend reserve.
    Invest,
    /// Deploy at most a caller-provided amount of available vault liquidity.
    InvestWithMaxAmount,
    /// Redeem vault shares for cTokens instead of the underlying token.
    RedeemInKind,
}

impl core::fmt::Display for KvaultInstruction {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{self:?}")
    }
}

/// Identify a Kvault instruction from its raw instruction data.
///
/// Returns `Some(variant)` if the first 8 bytes match a known discriminator,
/// `None` otherwise.
pub fn identify_instruction(data: &[u8]) -> Option<KvaultInstruction> {
    if data.len() < 8 {
        return None;
    }
    let mut disc = [0u8; 8];
    disc.copy_from_slice(&data[..8]);

    match disc {
        d if d == DEPOSIT => Some(KvaultInstruction::Deposit),
        d if d == DEPOSIT_WITH_MIN_SHARES_OUT => Some(KvaultInstruction::DepositWithMinSharesOut),
        d if d == BUY => Some(KvaultInstruction::Buy),
        d if d == BUY_WITH_MIN_SHARES_OUT => Some(KvaultInstruction::BuyWithMinSharesOut),
        d if d == WITHDRAW => Some(KvaultInstruction::Withdraw),
        d if d == SELL => Some(KvaultInstruction::Sell),
        d if d == WITHDRAW_FROM_AVAILABLE => Some(KvaultInstruction::WithdrawFromAvailable),
        d if d == INVEST => Some(KvaultInstruction::Invest),
        d if d == INVEST_WITH_MAX_AMOUNT => Some(KvaultInstruction::InvestWithMaxAmount),
        d if d == REDEEM_IN_KIND => Some(KvaultInstruction::RedeemInKind),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! check_disc {
        ($name:expr, $constant:ident) => {
            assert_eq!(
                compute_discriminator($name),
                $constant,
                "Discriminator mismatch for {}",
                $name
            );
        };
    }

    #[test]
    fn verify_all_discriminators() {
        check_disc!("deposit", DEPOSIT);
        check_disc!("deposit_with_min_shares_out", DEPOSIT_WITH_MIN_SHARES_OUT);
        check_disc!("buy", BUY);
        check_disc!("buy_with_min_shares_out", BUY_WITH_MIN_SHARES_OUT);
        check_disc!("withdraw", WITHDRAW);
        check_disc!("sell", SELL);
        check_disc!("withdraw_from_available", WITHDRAW_FROM_AVAILABLE);
        check_disc!("invest", INVEST);
        check_disc!("invest_with_max_amount", INVEST_WITH_MAX_AMOUNT);
        check_disc!("redeem_in_kind", REDEEM_IN_KIND);
    }
}
