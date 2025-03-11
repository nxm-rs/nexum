//! Common utilities for GlobalPlatform operations

/// Ensure a slice is exactly the expected length
pub fn check_length(data: &[u8], expected: usize) -> crate::Result<()> {
    if data.len() != expected {
        return Err(crate::error::Error::InvalidLength {
            expected,
            actual: data.len(),
        });
    }
    Ok(())
}

/// Ensure a slice is at least the minimum length
pub fn check_min_length(data: &[u8], min_length: usize) -> crate::Result<()> {
    if data.len() < min_length {
        return Err(crate::error::Error::InvalidLength {
            expected: min_length,
            actual: data.len(),
        });
    }
    Ok(())
}

pub mod tlv {
    /// Find a TLV value by tag
    pub fn find_tlv_value<'a>(data: &'a [u8], tag: u8) -> Option<&'a [u8]> {
        let mut index = 0;
        while index + 1 < data.len() {
            let current_tag = data[index];
            let len = data[index + 1] as usize;

            if current_tag == tag && index + 2 + len <= data.len() {
                return Some(&data[index + 2..index + 2 + len]);
            }

            index += 2 + len;
        }

        None
    }

    /// Extract all TLV values with the given tag
    pub fn find_all_tlv_values<'a>(data: &'a [u8], tag: u8) -> Vec<&'a [u8]> {
        let mut results = Vec::new();
        let mut index = 0;

        while index + 1 < data.len() {
            let current_tag = data[index];
            let len = data[index + 1] as usize;

            if current_tag == tag && index + 2 + len <= data.len() {
                results.push(&data[index + 2..index + 2 + len]);
            }

            index += 2 + len;
        }

        results
    }

    /// Build a TLV entry
    pub fn build_tlv(tag: u8, value: &[u8]) -> Vec<u8> {
        let mut result = Vec::with_capacity(2 + value.len());
        result.push(tag);
        result.push(value.len() as u8);
        result.extend_from_slice(value);
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn test_check_length() {
        let data = [1, 2, 3, 4];

        // Exact match should work
        assert!(check_length(&data, 4).is_ok());

        // Wrong length should fail
        assert!(check_length(&data, 5).is_err());
        assert!(check_length(&data, 3).is_err());
    }

    #[test]
    fn test_check_min_length() {
        let data = [1, 2, 3, 4];

        // Equal or greater than min should work
        assert!(check_min_length(&data, 4).is_ok());
        assert!(check_min_length(&data, 3).is_ok());
        assert!(check_min_length(&data, 1).is_ok());

        // Less than min should fail
        assert!(check_min_length(&data, 5).is_err());
    }

    #[test]
    fn test_find_tlv_value() {
        let data = hex!("4F07A000000003000084064143434F554E54");

        let aid = tlv::find_tlv_value(&data, 0x4F);
        assert_eq!(aid, Some(&hex!("A0000000030000")[..]));

        let label = tlv::find_tlv_value(&data, 0x84);
        assert_eq!(label, Some(&hex!("4143434F554E54")[..])); // "ACCOUNT" in ASCII

        let missing = tlv::find_tlv_value(&data, 0x50);
        assert_eq!(missing, None);
    }

    #[test]
    fn test_build_tlv() {
        let tag = 0x4F;
        let value = hex!("A0000000030000");

        let tlv_data = tlv::build_tlv(tag, &value);
        assert_eq!(tlv_data, hex!("4F07A0000000030000"));
    }
}
