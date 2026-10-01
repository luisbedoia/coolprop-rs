//! Minimal CBOR (RFC 8949) encoder for the JSON data model.
//!
//! Port of upstream's `CoolProp/dev/cbor_min.py`, so the slim fluid blob has
//! the same shape as the one CoolProp embeds: definite lengths everywhere and
//! floats always written as 64-bit doubles (no half/single shortening).

use serde_json::Value;

pub(crate) fn to_vec(value: &Value) -> Vec<u8> {
    let mut out = Vec::new();
    encode(value, &mut out);
    out
}

fn encode(value: &Value, out: &mut Vec<u8>) {
    match value {
        Value::Null => out.push(0xf6),
        Value::Bool(true) => out.push(0xf5),
        Value::Bool(false) => out.push(0xf4),
        Value::Number(n) => {
            if let Some(u) = n.as_u64() {
                head(0, u, out);
            } else if let Some(i) = n.as_i64() {
                // Negative integer: CBOR stores -1 - n.
                head(1, (-1 - i) as u64, out);
            } else {
                let f = n.as_f64().expect("JSON number is u64, i64 or f64");
                assert!(f.is_finite(), "non-finite float is not valid JSON: {f}");
                out.push(0xfb);
                out.extend_from_slice(&f.to_be_bytes());
            }
        }
        Value::String(s) => {
            head(3, s.len() as u64, out);
            out.extend_from_slice(s.as_bytes());
        }
        Value::Array(items) => {
            head(4, items.len() as u64, out);
            items.iter().for_each(|item| encode(item, out));
        }
        Value::Object(map) => {
            head(5, map.len() as u64, out);
            for (k, v) in map {
                head(3, k.len() as u64, out);
                out.extend_from_slice(k.as_bytes());
                encode(v, out);
            }
        }
    }
}

/// CBOR head: 3-bit major type + definite length/value `n`.
fn head(major: u8, n: u64, out: &mut Vec<u8>) {
    let mt = major << 5;
    match n {
        0..24 => out.push(mt | n as u8),
        24..0x100 => out.extend_from_slice(&[mt | 24, n as u8]),
        0x100..0x1_0000 => {
            out.push(mt | 25);
            out.extend_from_slice(&(n as u16).to_be_bytes());
        }
        0x1_0000..0x1_0000_0000 => {
            out.push(mt | 26);
            out.extend_from_slice(&(n as u32).to_be_bytes());
        }
        _ => {
            out.push(mt | 27);
            out.extend_from_slice(&n.to_be_bytes());
        }
    }
}
