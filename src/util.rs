
//! Internal utilities. These are not exposed!

/// Converts a sequence of bytes to a rust `String`. This assumes each byte to be between 0 and
/// 127. Bytes outside of this range are converted to ``
pub(crate) fn ascii_to_string(bytes: &[u8]) -> String {
    let mut string = String::with_capacity(bytes.len());

    for &byte in bytes.iter() {
        if (byte & 0x80) == 0x80 {
            // Not ascii, but start of a utf8 multibyte character!
            string.push('\0');
        } else {
            string.push(byte as char);
        }
    }

    return string;
}
