//! The DER reader, `surf_tls::der`: hostile input refused with its named error
//! (`story:tls-x509`, acceptance 2), each limit at one below, at and one above, the primitive
//! types, and no panic on any prefix or bit flip of a certificate.

#![forbid(unsafe_code)]

use surf_tls::der::{self, DEPTH_MAX, Error, LENGTH_BYTES_MAX, Reader, Time, tag};

use crate::alloc::peak_while;
use crate::x509_parse::{Spec, tlv};

/// `n` SEQUENCEs, each inside the one before, the innermost empty.
fn nested(n: usize) -> Vec<u8> {
    let mut value = vec![0x30, 0x00];
    for _ in 1..n {
        value = tlv(0x30, &value);
    }
    value
}

// ---------------------------------------------------------------------------------------------
// Acceptance 2: hostile DER.

#[test]
fn a_sequence_declaring_10_bytes_with_9_present_is_truncated() {
    let mut input = vec![0x30, 0x0A];
    input.extend_from_slice(&[0x05, 0x00, 0x05, 0x00, 0x05, 0x00, 0x05, 0x00, 0x05]);
    assert_eq!(der::parse(&input), Err(Error::Truncated));
}

#[test]
fn length_84_ff_ff_ff_ff_is_truncated_without_allocating_by_it() {
    let input = [0x30, 0x84, 0xFF, 0xFF, 0xFF, 0xFF];
    let (result, peak) = peak_while(|| der::parse(&input).map(|v| v.content().len()));
    assert_eq!(result, Err(Error::Truncated));
    assert!(peak < 64 * 1024, "peak {peak} bytes, under 64 KiB");
}

#[test]
fn length_80_is_the_indefinite_form() {
    assert_eq!(
        der::parse(&[0x30, 0x80, 0x00, 0x00]),
        Err(Error::IndefiniteLength)
    );
}

#[test]
fn length_81_05_is_not_minimal() {
    let input = [0x04, 0x81, 0x05, 1, 2, 3, 4, 5];
    assert_eq!(der::parse(&input), Err(Error::NonMinimalLength));
}

#[test]
fn a_long_length_with_a_leading_zero_is_not_minimal() {
    let mut input = vec![0x04, 0x82, 0x00, 0x80];
    input.extend_from_slice(&[0u8; 0x80]);
    assert_eq!(der::parse(&input), Err(Error::NonMinimalLength));
}

#[test]
fn length_81_80_is_the_shortest_form_of_128() {
    let mut input = vec![0x04, 0x81, 0x80];
    input.extend_from_slice(&[7u8; 0x80]);
    let value = der::parse(&input).expect("128 bytes");
    assert_eq!(value.content().len(), 128);
}

#[test]
fn five_length_bytes_are_too_long() {
    let input = [0x04, 0x85, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert_eq!(der::parse(&input), Err(Error::LengthTooLong));
}

#[test]
fn integer_00_01_is_not_minimal() {
    assert_eq!(
        der::parse(&[0x02, 0x02, 0x00, 0x01]),
        Err(Error::NonMinimalInteger)
    );
    assert_eq!(
        der::parse(&[0x02, 0x02, 0xFF, 0x80]),
        Err(Error::NonMinimalInteger)
    );
    assert_eq!(der::parse(&[0x02, 0x00]), Err(Error::EmptyInteger));
    let value = der::parse(&[0x02, 0x02, 0x00, 0x80]).expect("00 80 is minimal");
    assert_eq!(value.integer(), Ok(&[0x00, 0x80][..]));
    let value = der::parse(&[0x02, 0x02, 0xFF, 0x7F]).expect("FF 7F is minimal");
    assert_eq!(value.integer(), Ok(&[0xFF, 0x7F][..]));
}

#[test]
fn boolean_01_is_bad() {
    assert_eq!(der::parse(&[0x01, 0x01, 0x01]), Err(Error::BadBoolean));
    assert_eq!(
        der::parse(&[0x01, 0x02, 0xFF, 0xFF]),
        Err(Error::BadBoolean)
    );
    assert_eq!(
        der::parse(&[0x01, 0x01, 0x00]).unwrap().boolean(),
        Ok(false)
    );
    assert_eq!(der::parse(&[0x01, 0x01, 0xFF]).unwrap().boolean(), Ok(true));
}

#[test]
fn forty_nested_sequences_are_too_deep() {
    assert_eq!(der::parse(&nested(40)), Err(Error::TooDeep));
}

#[test]
fn one_byte_after_a_whole_certificate_is_trailing_data() {
    let mut input = Spec::p256().build();
    assert!(surf_tls::x509::Certificate::from_der(&input).is_ok());
    assert!(der::parse(&input).is_ok());
    input.push(0x00);
    assert_eq!(der::parse(&input), Err(Error::TrailingData));
    assert_eq!(
        surf_tls::x509::Certificate::from_der(&input).map(|_| ()),
        Err(surf_tls::x509::Error::Der(Error::TrailingData))
    );
}

// ---------------------------------------------------------------------------------------------
// The limits, each at one below, at and one above.

#[test]
fn depth_max_is_32() {
    assert_eq!(DEPTH_MAX, 32);
}

#[test]
fn nesting_at_one_below_and_at_depth_max_reads_and_one_above_is_too_deep() {
    assert!(der::parse(&nested(DEPTH_MAX - 1)).is_ok());
    assert!(der::parse(&nested(DEPTH_MAX)).is_ok());
    assert_eq!(der::parse(&nested(DEPTH_MAX + 1)), Err(Error::TooDeep));
}

#[test]
fn length_bytes_max_is_4() {
    assert_eq!(LENGTH_BYTES_MAX, 4);
}

/// The header alone, so that no test builds 16 MiB of content to show 4 length bytes.
#[test]
fn length_bytes_at_one_below_and_at_the_limit_read_and_one_above_is_too_long() {
    let three = der::header(&[0x04, 0x83, 0x01, 0x00, 0x00]).expect("3 length bytes");
    assert_eq!((three.header_len, three.length), (5, 0x01_0000));
    let four = der::header(&[0x04, 0x84, 0x01, 0x00, 0x00, 0x00]).expect("4 length bytes");
    assert_eq!((four.header_len, four.length), (6, 0x0100_0000));
    assert_eq!(
        der::header(&[0x04, 0x85, 0x01, 0x00, 0x00, 0x00, 0x00]),
        Err(Error::LengthTooLong)
    );
    // Four length bytes and no content: read as a length, then refused as truncated.
    assert_eq!(
        der::parse(&[0x04, 0x84, 0x01, 0x00, 0x00, 0x00]),
        Err(Error::Truncated)
    );
}

// ---------------------------------------------------------------------------------------------
// Tags, headers and readers.

#[test]
fn a_multi_byte_tag_is_unsupported() {
    assert_eq!(
        der::parse(&[0x1F, 0x81, 0x00, 0x00]),
        Err(Error::UnsupportedTag)
    );
    assert_eq!(der::parse(&[0x9F, 0x20, 0x00]), Err(Error::UnsupportedTag));
}

#[test]
fn a_constructed_string_is_unsupported() {
    // A constructed OCTET STRING (BER) holding one primitive one.
    assert_eq!(
        der::parse(&[0x24, 0x03, 0x04, 0x01, 0x00]),
        Err(Error::UnsupportedTag)
    );
    // SEQUENCE and SET in the primitive form.
    assert_eq!(der::parse(&[0x10, 0x00]), Err(Error::UnsupportedTag));
    assert_eq!(der::parse(&[0x11, 0x00]), Err(Error::UnsupportedTag));
}

#[test]
fn an_empty_input_or_a_lone_tag_is_truncated() {
    assert_eq!(der::parse(&[]), Err(Error::Truncated));
    assert_eq!(der::parse(&[0x30]), Err(Error::Truncated));
    assert_eq!(der::parse(&[0x30, 0x82, 0x01]), Err(Error::Truncated));
}

#[test]
fn a_bad_value_deep_inside_is_found() {
    let inner = tlv(0x30, &[0x02, 0x02, 0x00, 0x01]);
    let outer = tlv(0x30, &tlv(0x31, &inner));
    assert_eq!(der::parse(&outer), Err(Error::NonMinimalInteger));
}

#[test]
fn a_reader_names_the_missing_and_the_unexpected_tag() {
    let mut reader = Reader::new(&[0x05, 0x00]);
    assert_eq!(
        reader.expect(tag::INTEGER).map(|_| ()),
        Err(Error::UnexpectedTag {
            expected: tag::INTEGER,
            got: tag::NULL
        })
    );
    assert_eq!(reader.optional(tag::INTEGER), Ok(None));
    assert!(reader.expect(tag::NULL).is_ok());
    assert_eq!(
        reader.expect(tag::NULL).map(|_| ()),
        Err(Error::Missing {
            expected: tag::NULL
        })
    );
    assert_eq!(reader.finish(), Ok(()));
    let mut reader = Reader::new(&[0x05, 0x00, 0x05, 0x00]);
    reader.read().expect("first NULL");
    assert_eq!(reader.finish(), Err(Error::TrailingData));
}

#[test]
fn a_null_with_content_is_bad() {
    assert_eq!(der::parse(&[0x05, 0x01, 0x00]), Err(Error::BadNull));
}

#[test]
fn small_unsigned_reads_up_to_32_bits() {
    let v = |bytes: &[u8]| {
        der::parse(&tlv(0x02, bytes))
            .expect("an integer")
            .small_unsigned()
    };
    assert_eq!(v(&[0x00]), Ok(Some(0)));
    assert_eq!(v(&[0x7F]), Ok(Some(127)));
    assert_eq!(v(&[0x00, 0xFF, 0xFF, 0xFF, 0xFF]), Ok(Some(u32::MAX)));
    assert_eq!(v(&[0x01, 0x00, 0x00, 0x00, 0x00]), Ok(None));
    assert_eq!(v(&[0xFF]), Ok(None));
}

// ---------------------------------------------------------------------------------------------
// OBJECT IDENTIFIER, BIT STRING, strings.

#[test]
fn an_oid_reads_and_prints_dotted() {
    let san = der::parse(&[0x06, 0x03, 0x55, 0x1D, 0x11]).expect("2.5.29.17");
    assert_eq!(san.oid(), Ok(&[0x55, 0x1D, 0x11][..]));
    assert_eq!(der::oid_to_string(&[0x55, 0x1D, 0x11]), "2.5.29.17");
    assert_eq!(
        der::oid_to_string(&[0x2A, 0x86, 0x48, 0x86, 0xF7, 0x0D, 0x01, 0x01, 0x0B]),
        "1.2.840.113549.1.1.11"
    );
    assert_eq!(der::oid_to_string(&[0x00]), "0.0");
    assert_eq!(der::oid_to_string(&[0x88, 0x37]), "2.999");
    // An arc of 2^70, over 64 bits: 0x81 then nine bytes of 0x80, then 0x00.
    let mut big = vec![0x2B, 0x81];
    big.extend_from_slice(&[0x80; 9]);
    big.push(0x00);
    assert_eq!(der::oid_to_string(&big), "1.3.1180591620717411303424");
}

/// An arc that fits 128 bits prints in decimal, a longer one as `0x` and its bits in hex; the
/// first subidentifier over 128 bits is 2 and the rest less 80, in hex.
#[test]
fn an_arc_over_128_bits_prints_in_hex() {
    // 2^128 - 1: 0x83, then 17 bytes of 0xFF, then 0x7F.
    let mut max = vec![0x2B, 0x83];
    max.extend_from_slice(&[0xFF; 17]);
    max.push(0x7F);
    assert_eq!(
        der::oid_to_string(&max),
        "1.3.340282366920938463463374607431768211455"
    );
    // 2^128: 0x84, then 17 bytes of 0x80, then 0x00.
    let mut over = vec![0x84];
    over.extend_from_slice(&[0x80; 17]);
    over.push(0x00);
    assert_eq!(
        der::oid_to_string(&[&[0x2B][..], &over].concat()),
        "1.3.0x100000000000000000000000000000000"
    );
    // As the first subidentifier 2^128 is the arcs 2 and 2^128 - 80, which fits 128 bits.
    assert_eq!(
        der::oid_to_string(&over),
        "2.340282366920938463463374607431768211376"
    );
    // 2^132 as the first subidentifier: 2 and 2^132 - 80, over 128 bits.
    let mut huge = vec![0xC0];
    huge.extend_from_slice(&[0x80; 17]);
    huge.push(0x00);
    assert_eq!(
        der::oid_to_string(&[&huge[..], &[0x01]].concat()),
        format!("2.0x{}b0.1", "f".repeat(31))
    );
}

/// The steps of `oid_to_string` grow with the length of an arc, not its square: one arc of
/// `n` bytes and one of `2n`, the second at most twice the first and a constant.
#[test]
fn printing_an_oid_is_linear_in_the_length_of_an_arc() {
    let arc = |n: usize| {
        let mut content = vec![0x2B];
        content.extend(std::iter::repeat_n(0xFF, n - 1));
        content.push(0x7F);
        content
    };
    let (text, n_steps) = der::oid_to_string_counting(&arc(1000));
    let (_, two_n_steps) = der::oid_to_string_counting(&arc(2000));
    println!("oid_to_string: {n_steps} steps at 1000 bytes, {two_n_steps} at 2000");
    assert!(text.starts_with("1.3.0x"), "{}", &text[..8]);
    assert!(n_steps <= 4 * 1000, "{n_steps} steps for 1000 bytes");
    assert!(
        two_n_steps <= 2 * n_steps + 16,
        "{two_n_steps} steps at 2n against {n_steps} at n"
    );
}

#[test]
fn a_bad_oid_is_refused() {
    assert_eq!(der::parse(&[0x06, 0x00]), Err(Error::BadOid));
    assert_eq!(der::parse(&[0x06, 0x02, 0x55, 0x81]), Err(Error::BadOid));
    assert_eq!(
        der::parse(&[0x06, 0x03, 0x55, 0x80, 0x01]),
        Err(Error::BadOid)
    );
}

#[test]
fn a_bit_string_keeps_its_unused_bits_and_they_are_zero() {
    let usage = der::parse(&[0x03, 0x02, 0x07, 0x80]).expect("one bit");
    let bits = usage.bit_string().expect("a bit string");
    assert_eq!((bits.unused_bits, bits.bytes), (7, &[0x80][..]));
    assert_eq!(
        der::parse(&[0x03, 0x02, 0x07, 0x81]),
        Err(Error::BadBitString)
    );
    assert_eq!(
        der::parse(&[0x03, 0x02, 0x08, 0x00]),
        Err(Error::BadBitString)
    );
    assert_eq!(der::parse(&[0x03, 0x00]), Err(Error::BadBitString));
    assert_eq!(der::parse(&[0x03, 0x01, 0x01]), Err(Error::BadBitString));
    assert!(der::parse(&[0x03, 0x01, 0x00]).is_ok());
}

#[test]
fn octet_string_and_the_three_string_types_read() {
    let octets = der::parse(&[0x04, 0x02, 0xAB, 0xCD]).expect("octets");
    assert_eq!(octets.content(), &[0xAB, 0xCD]);
    let text = |t: u8, b: &[u8]| {
        der::parse(&tlv(t, b))
            .expect("a value")
            .string()
            .map(str::to_string)
    };
    assert_eq!(
        text(tag::UTF8_STRING, "grüß".as_bytes()),
        Ok("grüß".to_string())
    );
    assert_eq!(
        text(tag::PRINTABLE_STRING, b"Example CA 1"),
        Ok("Example CA 1".to_string())
    );
    assert_eq!(
        text(tag::IA5_STRING, b"a@b.example"),
        Ok("a@b.example".to_string())
    );
}

#[test]
fn a_character_its_string_type_does_not_allow_is_bad() {
    let bad = |t: u8, b: &[u8]| {
        der::parse(&tlv(t, b))
            .expect("a value")
            .string()
            .map(|_| ())
    };
    assert_eq!(bad(tag::UTF8_STRING, &[0xC3, 0x28]), Err(Error::BadString));
    assert_eq!(bad(tag::PRINTABLE_STRING, b"a@b"), Err(Error::BadString));
    assert_eq!(bad(tag::PRINTABLE_STRING, b"a*b"), Err(Error::BadString));
    assert_eq!(bad(tag::IA5_STRING, "ä".as_bytes()), Err(Error::BadString));
}

// ---------------------------------------------------------------------------------------------
// Times (RFC 5280 section 4.1.2.5).

fn time(t: u8, text: &str) -> Result<Time, Error> {
    der::parse(&tlv(t, text.as_bytes()))
        .expect("a value")
        .time()
}

#[test]
fn utc_time_and_generalized_time_read_with_seconds_and_z() {
    let utc = time(tag::UTC_TIME, "261122000000Z").expect("UTCTime");
    assert_eq!(utc, Time::from_utc(2026, 11, 22, 0, 0, 0).unwrap());
    assert_eq!(utc.to_string(), "2026-11-22T00:00:00Z");
    assert_eq!(utc.unix(), 1_795_305_600);
    let gen_time = time(tag::GENERALIZED_TIME, "29690503000001Z").expect("GeneralizedTime");
    assert_eq!(gen_time.to_string(), "2969-05-03T00:00:01Z");
    // YY of 50 and over is 19YY, under 50 is 20YY.
    assert_eq!(
        time(tag::UTC_TIME, "500101000000Z").unwrap().to_string(),
        "1950-01-01T00:00:00Z"
    );
    assert_eq!(
        time(tag::UTC_TIME, "491231235959Z").unwrap().to_string(),
        "2049-12-31T23:59:59Z"
    );
    assert_eq!(
        time(tag::UTC_TIME, "700101000001Z").unwrap(),
        Time::from_unix(1)
    );
    assert_eq!(
        time(tag::GENERALIZED_TIME, "19000101000000Z")
            .unwrap()
            .to_string(),
        "1900-01-01T00:00:00Z"
    );
}

#[test]
fn utc_time_without_seconds_is_bad() {
    assert_eq!(time(tag::UTC_TIME, "2501010000Z"), Err(Error::BadTime));
}

#[test]
fn february_30_is_bad() {
    assert_eq!(time(tag::UTC_TIME, "250230000000Z"), Err(Error::BadTime));
    assert_eq!(time(tag::UTC_TIME, "230229000000Z"), Err(Error::BadTime));
    assert!(
        time(tag::UTC_TIME, "240229000000Z").is_ok(),
        "2024 is a leap year"
    );
    assert_eq!(
        time(tag::GENERALIZED_TIME, "21000229000000Z"),
        Err(Error::BadTime)
    );
    assert!(time(tag::GENERALIZED_TIME, "20000229000000Z").is_ok());
}

#[test]
fn times_in_other_forms_are_bad() {
    for text in [
        "250101000000",      // no Z
        "250101000000+0100", // an offset
        "2501010000000Z",    // one digit too many
        "251301000000Z",     // month 13
        "250100000000Z",     // day 0
        "250101240000Z",     // hour 24
        "250101006000Z",     // minute 60
        "250101000060Z",     // second 60
        "2501010000-1Z",     // not a digit
    ] {
        assert_eq!(time(tag::UTC_TIME, text), Err(Error::BadTime), "{text}");
    }
    for text in [
        "20250101000000.5Z", // a fraction
        "20250101000000",    // no Z
        "202501010000Z",     // no seconds
    ] {
        assert_eq!(
            time(tag::GENERALIZED_TIME, text),
            Err(Error::BadTime),
            "{text}"
        );
    }
}

// ---------------------------------------------------------------------------------------------
// No panic on any input.

#[test]
fn every_prefix_and_every_bit_flip_of_a_certificate_reads_or_is_refused() {
    let cert = Spec::p256().build();
    let mut refused = 0;
    for end in 0..cert.len() {
        refused += usize::from(surf_tls::x509::Certificate::from_der(&cert[..end]).is_err());
    }
    assert_eq!(refused, cert.len(), "every proper prefix is refused");
    let mut copy = cert.clone();
    for i in 0..copy.len() {
        for bit in 0..8 {
            copy[i] ^= 1 << bit;
            let _ = der::parse(&copy);
            let _ = surf_tls::x509::Certificate::from_der(&copy);
            copy[i] ^= 1 << bit;
        }
    }
    assert_eq!(copy, cert);
}
