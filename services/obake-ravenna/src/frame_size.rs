// SPDX-FileCopyrightText: Copyright 2026 Greg Wuller
// SPDX-License-Identifier: MIT

//! Legal AES67 / RAVENNA sample rates and frame sizes.

/// AES67 and RAVENNA media clock rates the daemon accepts as a graph rate.
///
/// AES67: 48 kHz required; 44.1 kHz and 96 kHz optional.
/// RAVENNA: 44.1, 48, 96, 192, 384 kHz, plus the common 88.2 / 176.4 /
/// 352.8 kHz pull-up doubles used by many endpoints.
pub const LEGAL_RATES: &[u32] = &[
    44_100, 48_000, 88_200, 96_000, 176_400, 192_000, 352_800, 384_000,
];

/// Allowed packet times as (numerator, denominator) milliseconds of media
/// time: num/den ms. Includes AES67 sizes plus RAVENNA 2/3 ms and 4/3 ms.
const LEGAL_PTIME_NUM: &[(u32, u32)] = &[
    (1, 8), // 0.125 ms
    (1, 4), // 0.250 ms
    (1, 3), // ~0.333 ms
    (2, 3), // ~0.667 ms
    (1, 1), // 1 ms
    (4, 3), // ~1.333 ms
    (4, 1), // 4 ms
];

/// Absolute sample counts that are legal at 48 kHz under [`LEGAL_PTIME_NUM`].
/// Used as a fallback on the 44.1 kHz family, where those ptimes are not an
/// integer number of samples (common RAVENNA / AES67 endpoint practice).
const NOMINAL_FRAME_SIZES: &[u32] = &[6, 12, 16, 32, 48, 64, 192];

/// Returns true when `rate` is an AES67 or RAVENNA media clock rate.
pub fn is_legal_rate(rate: u32) -> bool {
    LEGAL_RATES.contains(&rate)
}

/// True for 44.1 kHz and its doubles, where standard ptimes are not integer.
fn is_pull_rate(rate: u32) -> bool {
    matches!(rate, 44_100 | 88_200 | 176_400 | 352_800)
}

/// Returns true when `frame_size` samples at `rate` Hz is an allowed packet.
///
/// On 48 kHz-family rates, the packet time must match an AES67/RAVENNA
/// ptime exactly. On the 44.1 kHz family, the same absolute sample counts
/// that are legal at 48 kHz are accepted.
pub fn is_legal(frame_size: u32, rate: u32) -> bool {
    if frame_size == 0 || rate == 0 || !is_legal_rate(rate) {
        return false;
    }
    if exact_ptime_match(frame_size, rate) {
        return true;
    }
    is_pull_rate(rate) && NOMINAL_FRAME_SIZES.contains(&frame_size)
}

/// Exact `frame_size / rate == num/(den*1000)` match for a legal ptime.
fn exact_ptime_match(frame_size: u32, rate: u32) -> bool {
    for &(num, den) in LEGAL_PTIME_NUM {
        // frame_size * den * 1000 == rate * num
        let left = u64::from(frame_size) * u64::from(den) * 1000;
        let right = u64::from(rate) * u64::from(num);
        if left == right {
            return true;
        }
    }
    false
}

/// Packet time in milliseconds as a float for SDP `a=ptime`.
pub fn ptime_ms(frame_size: u32, rate: u32) -> Option<f64> {
    if !is_legal(frame_size, rate) {
        return None;
    }
    Some(f64::from(frame_size) * 1000.0 / f64::from(rate))
}

/// Frame sizes that are legal at `rate`, or empty when the rate is unknown.
pub fn legal_frame_sizes(rate: u32) -> Vec<u32> {
    if !is_legal_rate(rate) {
        return Vec::new();
    }
    if is_pull_rate(rate) {
        return NOMINAL_FRAME_SIZES.to_vec();
    }
    let mut sizes = Vec::new();
    for &(num, den) in LEGAL_PTIME_NUM {
        let denom = u64::from(den) * 1000;
        let numer = u64::from(rate) * u64::from(num);
        if numer % denom == 0 {
            sizes.push((numer / denom) as u32);
        }
    }
    sizes
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_aes67_and_ravenna_rates() {
        for rate in LEGAL_RATES {
            assert!(is_legal_rate(*rate), "expected {rate} legal");
        }
        assert!(!is_legal_rate(32_000));
        assert!(!is_legal_rate(50_000));
    }

    #[test]
    fn accepts_aes67_and_ravenna_sizes_at_48k() {
        for size in [6, 12, 16, 32, 48, 64, 192] {
            assert!(is_legal(size, 48_000), "expected {size} legal");
        }
    }

    #[test]
    fn accepts_scaled_sizes_at_96k_and_192k() {
        for size in [12, 24, 32, 64, 96, 128, 384] {
            assert!(is_legal(size, 96_000), "expected {size} legal at 96k");
        }
        assert!(is_legal(192, 192_000));
        assert!(is_legal(384, 384_000));
        // 48 samples is 0.5 ms at 96 kHz — not an allowed ptime.
        assert!(!is_legal(48, 96_000));
    }

    #[test]
    fn accepts_nominal_sizes_at_44k1_family() {
        for rate in [44_100, 88_200, 176_400, 352_800] {
            for size in NOMINAL_FRAME_SIZES {
                assert!(
                    is_legal(*size, rate),
                    "expected {size} legal at {rate}"
                );
            }
            assert!(!is_legal(40, rate));
        }
    }

    #[test]
    fn rejects_40_at_48k() {
        assert!(!is_legal(40, 48_000));
    }

    #[test]
    fn rejects_illegal_rate_even_with_matching_ptime() {
        // 32 samples is 1 ms at 32 kHz, but 32 kHz is not AES67/RAVENNA.
        assert!(!is_legal(32, 32_000));
    }

    #[test]
    fn ptime_for_48_and_64() {
        let p48 = ptime_ms(48, 48_000).unwrap();
        assert!((p48 - 1.0).abs() < 1e-9);
        let p64 = ptime_ms(64, 48_000).unwrap();
        assert!((p64 - 4.0 / 3.0).abs() < 1e-9);
        let p32 = ptime_ms(32, 48_000).unwrap();
        assert!((p32 - 2.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn legal_frame_sizes_lists_48k_and_96k() {
        assert_eq!(
            legal_frame_sizes(48_000),
            vec![6, 12, 16, 32, 48, 64, 192]
        );
        assert_eq!(
            legal_frame_sizes(96_000),
            vec![12, 24, 32, 64, 96, 128, 384]
        );
        assert_eq!(legal_frame_sizes(44_100), NOMINAL_FRAME_SIZES.to_vec());
        assert!(legal_frame_sizes(32_000).is_empty());
    }
}
