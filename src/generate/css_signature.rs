//! The single owner of "how are several values folded into one identity?".
//!
//! A digest fed variable-length fields pasted end to end is not a function of those fields, only
//! of their concatenation: `ANNA` + `BELL` and `ANN` + `ABELL` flatten to the same bytes. The
//! sharpest case needs no clever substring — when two optional fields are appended in sequence,
//! an item carrying only the first and an item carrying only the second contribute byte-for-byte
//! identical text, because absence is encoded as nothing and nothing is indistinguishable from
//! not-reached-yet.
//!
//! Every field is therefore terminated, and an empty slot still writes its marker, so an absent
//! value and a present-but-empty one are different bytes. Callers may then fold whatever they
//! like without having to reason about whether their payloads can be confused.

use crate::model::Styles;
use sha2::{Digest, Sha256};

/// Terminates a property name, separating it from the value that follows.
const KEY_END: u8 = 0;
/// Terminates a value, so no value can run into whatever is folded in next.
const VALUE_END: u8 = 0xff;
/// Opens a slot, written whether or not the slot is filled, so absence is a symbol of its own.
const SLOT: u8 = 0xfe;

#[derive(Default)]
pub struct Signature(Sha256);

impl Signature {
    pub fn new() -> Self {
        Self::default()
    }

    /// Folds in a name and the value it carries.
    pub fn pair(&mut self, key: &str, value: &str) {
        self.0.update(key.as_bytes());
        self.0.update([KEY_END]);
        self.0.update(value.as_bytes());
        self.0.update([VALUE_END]);
    }

    /// Folds in a value that stands on its own.
    pub fn value(&mut self, value: &str) {
        self.0.update(value.as_bytes());
        self.0.update([VALUE_END]);
    }

    /// Opens a slot that may or may not hold anything.
    pub fn slot(&mut self) {
        self.0.update([SLOT]);
    }

    pub fn styles(&mut self, styles: &Styles) {
        for (key, value) in styles {
            self.pair(key, value);
        }
    }

    pub fn finish(self) -> String {
        hex::encode(self.0.finalize())
    }
}
