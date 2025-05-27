//! Utility functions for PC/SC operations

/// Match an ATR against a pattern with an optional mask
///
/// If a mask is provided, only the bits set in the mask are compared.
pub(crate) fn match_atr(atr: &[u8], pattern: &[u8], mask: Option<&[u8]>) -> bool {
    // If pattern is longer than ATR, it can't match
    if pattern.len() > atr.len() {
        return false;
    }

    if let Some(mask) = mask {
        // Mask must be at least as long as pattern
        if mask.len() < pattern.len() {
            return false;
        }

        // Compare with mask
        for i in 0..pattern.len() {
            if (atr[i] & mask[i]) != (pattern[i] & mask[i]) {
                return false;
            }
        }
    } else {
        // Direct compare without mask
        for i in 0..pattern.len() {
            if atr[i] != pattern[i] {
                return false;
            }
        }
    }

    true
}
