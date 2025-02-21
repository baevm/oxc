use super::{ESTree, ESTreeSerializer, Formatter, Serializer, SerializerPrivate};

/// Trait for sequence serializers.
pub trait SequenceSerializer {
    /// Serialize sequence entry.
    fn serialize_element<T: ESTree + ?Sized>(&mut self, value: &T);

    /// Finish serializing sequence.
    fn end(self);
}

/// Serializer for sequences.
///
/// This is returned by `ESTreeSerializer::serialize_sequence`.
pub struct ESTreeSequenceSerializer<'s, F: Formatter> {
    /// Serializer
    serializer: &'s mut ESTreeSerializer<F>,
    /// State of sequence.
    /// Starts as `SequenceState::Empty`, transitions to `SequenceState::HasEntries` on first entry.
    state: SequenceState,
}

impl<'s, F: Formatter> ESTreeSequenceSerializer<'s, F> {
    /// Create new [`ESTreeSequenceSerializer`].
    pub(super) fn new(mut serializer: &'s mut ESTreeSerializer<F>) -> Self {
        serializer.buffer_mut().push_ascii_byte(b'[');
        Self { serializer, state: SequenceState::Empty }
    }
}

impl<F: Formatter> SequenceSerializer for ESTreeSequenceSerializer<'_, F> {
    /// Serialize sequence entry.
    fn serialize_element<T: ESTree + ?Sized>(&mut self, value: &T) {
        let (buffer, formatter) = self.serializer.buffer_and_formatter_mut();
        if self.state == SequenceState::Empty {
            self.state = SequenceState::HasEntries;
            formatter.before_first_element(buffer);
        } else {
            buffer.push_ascii_byte(b',');
            formatter.before_later_element(buffer);
        }

        value.serialize(&mut *self.serializer);
    }

    /// Finish serializing sequence.
    fn end(mut self) {
        let (buffer, formatter) = self.serializer.buffer_and_formatter_mut();
        if self.state == SequenceState::HasEntries {
            formatter.after_last_element(buffer);
        }
        buffer.push_ascii_byte(b']');
    }
}

/// State of [`ESTreeSequenceSerializer`].
#[derive(Clone, Copy, PartialEq, Eq)]
enum SequenceState {
    Empty,
    HasEntries,
}

/// [`ESTree`] implementation for slices.
impl<T: ESTree> ESTree for &[T] {
    fn serialize<S: Serializer>(&self, serializer: S) {
        let mut seq = serializer.serialize_sequence();
        for element in *self {
            seq.serialize_element(element);
        }
        seq.end();
    }
}

/// [`ESTree`] implementation for arrays.
impl<T: ESTree, const N: usize> ESTree for [T; N] {
    fn serialize<S: Serializer>(&self, serializer: S) {
        let mut seq = serializer.serialize_sequence();
        for element in self {
            seq.serialize_element(element);
        }
        seq.end();
    }
}

#[cfg(test)]
mod tests {
    use super::super::{CompactSerializer, PrettySerializer, StructSerializer};
    use super::*;

    #[test]
    fn serialize_sequence() {
        struct Foo<'a> {
            none: &'a [&'a str],
            one: &'a [&'a str],
            two: [&'a str; 2],
        }

        impl ESTree for Foo<'_> {
            fn serialize<S: Serializer>(&self, serializer: S) {
                let mut state = serializer.serialize_struct();
                state.serialize_field("none", &self.none);
                state.serialize_field("one", &self.one);
                state.serialize_field("two", &self.two);
                state.end();
            }
        }

        let foo = Foo { none: &[], one: &["one"], two: ["two one", "two two"] };

        let mut serializer = CompactSerializer::new();
        foo.serialize(&mut serializer);
        let s = serializer.into_string();
        assert_eq!(&s, r#"{"none":[],"one":["one"],"two":["two one","two two"]}"#);

        let mut serializer = PrettySerializer::new();
        foo.serialize(&mut serializer);
        let s = serializer.into_string();
        assert_eq!(
            &s,
            r#"{
  "none": [],
  "one": [
    "one"
  ],
  "two": [
    "two one",
    "two two"
  ]
}"#
        );
    }
}
