//! Common utilities for GlobalPlatform operations

pub mod tlv {
    use bytes::Bytes;
    use iso7816_tlv::simple::Tlv;

    /// Parse all TLVs from the input data, consuming the input
    pub fn parse_tlvs(data: &Bytes) -> Vec<Tlv> {
        Tlv::parse_all(data)
    }

    /// Find a TLV value by tag, consuming the input data
    pub fn find_tlv_value(data: Bytes, tag: u8) -> Option<Bytes> {
        let mut current_data = data.as_ref();

        // Parse TLVs until we find the matching tag or run out of data
        while !current_data.is_empty() {
            let (tlv_result, remaining) = Tlv::parse(current_data);

            match tlv_result {
                Ok(tlv) => {
                    // Convert the Tag to u8 for comparison
                    let tlv_tag: u8 = tlv.tag().into();
                    if tlv_tag == tag {
                        // Return the owned value
                        return Some(Bytes::copy_from_slice(tlv.value()));
                    }
                    // Move to the next TLV
                    current_data = remaining;
                }
                Err(_) => {
                    // Parsing error - stop processing
                    return None;
                }
            }
        }

        None
    }

    /// Extract all TLV values with the given tag, consuming the input data
    pub fn find_all_tlv_values(data: Bytes, tag: u8) -> Vec<Bytes> {
        // Use parse_all to get all TLVs
        let tlvs = Tlv::parse_all(&data);

        // Filter TLVs by tag and convert values to Bytes
        tlvs.into_iter()
            .filter_map(|tlv| {
                let tlv_tag: u8 = tlv.tag().into();
                if tlv_tag == tag {
                    Some(Bytes::copy_from_slice(tlv.value()))
                } else {
                    None
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::Bytes;
    use hex_literal::hex;

    #[test]
    fn test_find_tlv_value() {
        let data = Bytes::from(hex!("4F07A000000003000084074143434F554E54").to_vec());

        let aid = tlv::find_tlv_value(data.clone(), 0x4F);
        assert_eq!(aid, Some(Bytes::from(hex!("A0000000030000").to_vec())));

        let label = tlv::find_tlv_value(data.clone(), 0x84);
        assert_eq!(label, Some(Bytes::from(hex!("4143434F554E54").to_vec()))); // "ACCOUNT" in ASCII

        let missing = tlv::find_tlv_value(data.clone(), 0x50);
        assert_eq!(missing, None);
    }

    #[test]
    fn test_parse_tlvs() {
        let data = Bytes::from(hex!("4F07A000000003000084074143434F554E54").to_vec());

        let tlvs = tlv::parse_tlvs(&data);
        assert_eq!(tlvs.len(), 2);

        // Check first TLV - correctly use Tag's into() method for conversion to u8
        let tag_value: u8 = tlvs[0].tag().into();
        assert_eq!(tag_value, 0x4F);
        assert_eq!(tlvs[0].value(), &hex!("A0000000030000")[..]);

        // Check second TLV - correctly use Tag's into() method for conversion to u8
        let tag_value: u8 = tlvs[1].tag().into();
        assert_eq!(tag_value, 0x84);
        assert_eq!(tlvs[1].value(), &hex!("4143434F554E54")[..]); // "ACCOUNT" in ASCII
    }
}
