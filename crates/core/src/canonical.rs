use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::SchemaRef;

/// GlassVM's typed structured value foundation for canonical machine data.
///
/// This representation is intentionally narrower than `serde_json::Value`:
/// byte strings are distinct from text, maps have string keys, and floating
/// point values have explicit publication rules. The encoder below is the
/// GlassVM canonical CBOR implementation; it does not delegate canonical
/// ordering or numeric choices to a CBOR library.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StructuredValue {
    Null,
    Bool(bool),
    Unsigned(u64),
    Signed(i64),
    Float(f64),
    Text(String),
    Bytes(Vec<u8>),
    Array(Vec<Self>),
    Map(BTreeMap<String, Self>),
}

impl StructuredValue {
    pub fn canonical_cbor_bytes(&self) -> Result<Vec<u8>, String> {
        canonical_cbor_bytes(self)
    }
}

/// Encode a structured value using the GlassVM canonical CBOR representation.
pub fn canonical_cbor_bytes(value: &StructuredValue) -> Result<Vec<u8>, String> {
    let mut output = Vec::new();
    write_value(value, &mut output)?;
    Ok(output)
}

/// Encode a schema reference as an explicit canonical CBOR map containing its
/// ID and version fields.
pub fn canonical_schema_ref_bytes(schema: &SchemaRef) -> Result<Vec<u8>, String> {
    let mut version = BTreeMap::new();
    version.insert(
        "major".to_owned(),
        StructuredValue::Unsigned(schema.version.major as u64),
    );
    version.insert(
        "minor".to_owned(),
        StructuredValue::Unsigned(schema.version.minor as u64),
    );
    version.insert(
        "patch".to_owned(),
        StructuredValue::Unsigned(schema.version.patch as u64),
    );

    let mut reference = BTreeMap::new();
    reference.insert("id".to_owned(), StructuredValue::Text(schema.id.clone()));
    reference.insert("version".to_owned(), StructuredValue::Map(version));
    canonical_cbor_bytes(&StructuredValue::Map(reference))
}

fn write_value(value: &StructuredValue, output: &mut Vec<u8>) -> Result<(), String> {
    match value {
        StructuredValue::Null => output.push(0xf6),
        StructuredValue::Bool(false) => output.push(0xf4),
        StructuredValue::Bool(true) => output.push(0xf5),
        StructuredValue::Unsigned(value) => write_argument(0, *value, output),
        StructuredValue::Signed(value) if *value >= 0 => write_argument(0, *value as u64, output),
        StructuredValue::Signed(value) => write_argument(1, (!*value) as u64, output),
        StructuredValue::Float(value) => {
            if !value.is_finite() {
                return Err("canonical structured values reject non-finite floats".into());
            }
            output.push(0xfb);
            let normalized = if *value == 0.0 { 0.0 } else { *value };
            output.extend_from_slice(&normalized.to_bits().to_be_bytes());
        }
        StructuredValue::Text(value) => write_bytes(3, value.as_bytes(), output),
        StructuredValue::Bytes(value) => write_bytes(2, value, output),
        StructuredValue::Array(values) => {
            write_argument(4, values.len() as u64, output);
            for value in values {
                write_value(value, output)?;
            }
        }
        StructuredValue::Map(values) => {
            write_argument(5, values.len() as u64, output);
            let mut entries = values
                .iter()
                .map(|(key, value)| {
                    let mut encoded_key = Vec::new();
                    write_bytes(3, key.as_bytes(), &mut encoded_key);
                    (encoded_key, value)
                })
                .collect::<Vec<_>>();
            entries.sort_by(|(left, _), (right, _)| {
                left.len().cmp(&right.len()).then_with(|| left.cmp(right))
            });
            for (encoded_key, value) in entries {
                output.extend_from_slice(&encoded_key);
                write_value(value, output)?;
            }
        }
    }
    Ok(())
}

fn write_bytes(major: u8, bytes: &[u8], output: &mut Vec<u8>) {
    write_argument(major, bytes.len() as u64, output);
    output.extend_from_slice(bytes);
}

fn write_argument(major: u8, argument: u64, output: &mut Vec<u8>) {
    debug_assert!(major <= 7);
    if argument <= 23 {
        output.push((major << 5) | argument as u8);
    } else if argument <= u8::MAX as u64 {
        output.push((major << 5) | 24);
        output.push(argument as u8);
    } else if argument <= u16::MAX as u64 {
        output.push((major << 5) | 25);
        output.extend_from_slice(&(argument as u16).to_be_bytes());
    } else if argument <= u32::MAX as u64 {
        output.push((major << 5) | 26);
        output.extend_from_slice(&(argument as u32).to_be_bytes());
    } else {
        output.push((major << 5) | 27);
        output.extend_from_slice(&argument.to_be_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SchemaVersion;

    #[test]
    fn canonical_maps_use_encoded_key_length_then_bytes() {
        let mut values = BTreeMap::new();
        values.insert("aa".to_owned(), StructuredValue::Unsigned(2));
        values.insert("z".to_owned(), StructuredValue::Unsigned(1));

        assert_eq!(
            canonical_cbor_bytes(&StructuredValue::Map(values)).unwrap(),
            vec![0xa2, 0x61, b'z', 0x01, 0x62, b'a', b'a', 0x02]
        );
    }

    #[test]
    fn bytes_text_and_arrays_have_distinct_cbor_shapes() {
        assert_eq!(
            canonical_cbor_bytes(&StructuredValue::Text("ab".into())).unwrap(),
            vec![0x62, b'a', b'b']
        );
        assert_eq!(
            canonical_cbor_bytes(&StructuredValue::Bytes(b"ab".to_vec())).unwrap(),
            vec![0x42, b'a', b'b']
        );
        assert_eq!(
            canonical_cbor_bytes(&StructuredValue::Array(vec![StructuredValue::Null])).unwrap(),
            vec![0x81, 0xf6]
        );
    }

    #[test]
    fn floats_are_binary64_signed_zero_is_normalized_and_nonfinite_values_fail() {
        let positive_zero = canonical_cbor_bytes(&StructuredValue::Float(0.0)).unwrap();
        let negative_zero = canonical_cbor_bytes(&StructuredValue::Float(-0.0)).unwrap();
        assert_eq!(positive_zero, negative_zero);
        assert_eq!(positive_zero, vec![0xfb, 0, 0, 0, 0, 0, 0, 0, 0]);

        assert!(canonical_cbor_bytes(&StructuredValue::Float(f64::INFINITY)).is_err());
        assert!(canonical_cbor_bytes(&StructuredValue::Float(f64::NEG_INFINITY)).is_err());
        assert!(canonical_cbor_bytes(&StructuredValue::Float(f64::NAN)).is_err());
    }

    #[test]
    fn signed_integer_boundaries_use_canonical_negative_encoding() {
        assert_eq!(
            canonical_cbor_bytes(&StructuredValue::Signed(-1)).unwrap(),
            vec![0x20]
        );
        assert_eq!(
            canonical_cbor_bytes(&StructuredValue::Signed(i64::MIN)).unwrap(),
            vec![0x3b, 0x7f, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff]
        );
    }

    #[test]
    fn schema_refs_encode_explicit_id_and_version_fields() {
        let schema = SchemaRef::new("glassvm.input.digital-key", SchemaVersion::new(1, 2, 3));
        let encoded = canonical_schema_ref_bytes(&schema).unwrap();
        assert_eq!(encoded[0], 0xa2);
        assert!(
            encoded
                .windows(3)
                .any(|window| window == [0x62, b'i', b'd'])
        );
        assert!(
            encoded
                .windows(8)
                .any(|window| { window == [0x67, b'v', b'e', b'r', b's', b'i', b'o', b'n'] })
        );
    }
}
