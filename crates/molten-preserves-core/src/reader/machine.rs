//! The decoder state machine: one explicit frame per open compound or prefix.

type Failure = super::failure::DecodeError;

/// Byte range `[start, end)` of one complete value in the input, annotations included.
type Span = (usize, usize);

enum Builder {
    Record {
        label: Option<crate::model::PackedValue>,
        fields: alloc::vec::Vec<crate::model::PackedValue>,
    },
    Sequence(alloc::vec::Vec<crate::model::PackedValue>),
    Set(crate::model::collections::PackedSet),
    Dictionary {
        dictionary: crate::model::collections::PackedDictionary,
        key: Option<(alloc::vec::Vec<u8>, crate::model::PackedValue)>,
    },
    Embedded,
    Annotation,
}

struct Frame {
    builder: Builder,
    /// Where this value began, including any annotations before it.
    start: usize,
    children: u64,
}

struct Decoder<'a> {
    cursor: super::input::Cursor<'a>,
    limits: super::DecodeLimits,
    policy: super::AnnotationPolicy,
    frames: alloc::vec::Vec<Frame>,
    nodes: u64,
    /// Start of a value whose annotations were just skipped; the next token continues it.
    annotated_start: Option<usize>,
}

pub(super) fn decode(
    bytes: &[u8],
    limits: &super::DecodeLimits,
    policy: super::AnnotationPolicy,
) -> Result<crate::model::PackedValue, Failure> {
    if exceeds(bytes.len(), limits.max_bytes) {
        return Err(Failure::InputTooLarge {
            limit: limits.max_bytes,
        });
    }
    let mut decoder = Decoder {
        cursor: super::input::Cursor::new(bytes),
        limits: *limits,
        policy,
        frames: alloc::vec::Vec::new(),
        nodes: 0,
        annotated_start: None,
    };
    let root = decoder.run()?;
    if decoder.cursor.is_at_end() {
        Ok(root)
    } else {
        Err(Failure::TrailingBytes)
    }
}

/// True when `count` is past `limit`; a limit beyond `usize` is never passed.
fn exceeds(count: usize, limit: u64) -> bool {
    usize::try_from(limit).is_ok_and(|limit| count > limit)
}

impl Decoder<'_> {
    fn run(&mut self) -> Result<crate::model::PackedValue, Failure> {
        loop {
            let token_start = self.cursor.position();
            let byte = self.cursor.next_byte().ok_or(Failure::Truncated)?;
            let tag = crate::tags::Tag::from_byte(byte).ok_or(Failure::InvalidTag(byte))?;
            let annotated_start = self.annotated_start.take();
            let (value, start) = if tag == crate::tags::Tag::End {
                if annotated_start.is_some() {
                    return Err(Failure::MissingValue);
                }
                self.close()?
            } else {
                let start = annotated_start.unwrap_or(token_start);
                match self.open(tag, start)? {
                    Some(value) => (value, start),
                    None => continue,
                }
            };
            if let Some(root) = self.deliver(value, start)? {
                return Ok(root);
            }
        }
    }

    /// Starts the value tagged `tag`: atoms complete at once, compounds and prefixes open a frame.
    fn open(&mut self, tag: crate::tags::Tag, start: usize) -> Result<Option<crate::model::PackedValue>, Failure> {
        self.nodes = self.nodes.saturating_add(1);
        if self.nodes > self.limits.max_nodes {
            return Err(Failure::NodesExceeded {
                limit: self.limits.max_nodes,
            });
        }
        let builder = match tag {
            crate::tags::Tag::False => return Ok(Some(crate::model::PackedValue::Boolean(false))),
            crate::tags::Tag::True => return Ok(Some(crate::model::PackedValue::Boolean(true))),
            crate::tags::Tag::Ieee754
            | crate::tags::Tag::SignedInteger
            | crate::tags::Tag::String
            | crate::tags::Tag::ByteString
            | crate::tags::Tag::Symbol => return self.cursor.atom(tag).map(Some),
            crate::tags::Tag::End => return Err(Failure::UnexpectedEnd),
            crate::tags::Tag::Record => Builder::Record {
                label: None,
                fields: alloc::vec::Vec::new(),
            },
            crate::tags::Tag::Sequence => Builder::Sequence(alloc::vec::Vec::new()),
            crate::tags::Tag::Set => Builder::Set(crate::model::collections::PackedSet::new()),
            crate::tags::Tag::Dictionary => Builder::Dictionary {
                dictionary: crate::model::collections::PackedDictionary::new(),
                key: None,
            },
            crate::tags::Tag::Embedded => Builder::Embedded,
            crate::tags::Tag::Annotation => match self.policy {
                super::AnnotationPolicy::Reject => return Err(Failure::Annotation),
                super::AnnotationPolicy::Skip => Builder::Annotation,
            },
        };
        if exceeds(self.frames.len().saturating_add(1), self.limits.max_depth) {
            return Err(Failure::DepthExceeded {
                limit: self.limits.max_depth,
            });
        }
        self.frames.push(Frame {
            builder,
            start,
            children: 0,
        });
        Ok(None)
    }

    /// Closes the innermost compound at an end marker.
    fn close(&mut self) -> Result<(crate::model::PackedValue, usize), Failure> {
        let frame = self.frames.pop().ok_or(Failure::UnexpectedEnd)?;
        let value = match frame.builder {
            Builder::Record {
                label: Some(label),
                fields,
            } => crate::model::PackedValue::record(label, fields),
            Builder::Record { label: None, .. } => return Err(Failure::MissingRecordLabel),
            Builder::Sequence(items) => crate::model::PackedValue::Sequence(items),
            Builder::Set(set) => crate::model::PackedValue::Set(set),
            Builder::Dictionary { key: Some(_), .. } | Builder::Embedded | Builder::Annotation => {
                return Err(Failure::MissingValue);
            }
            Builder::Dictionary { dictionary, .. } => crate::model::PackedValue::Dictionary(dictionary),
        };
        Ok((value, frame.start))
    }

    /// Hands a completed value to its parent frame; returns the root once it completes.
    fn deliver(
        &mut self,
        mut value: crate::model::PackedValue,
        mut start: usize,
    ) -> Result<Option<crate::model::PackedValue>, Failure> {
        loop {
            let Some(frame) = self.frames.last_mut() else {
                return Ok(Some(value));
            };
            match frame.builder {
                Builder::Embedded => {
                    start = frame.start;
                    value = crate::model::PackedValue::embedded(value);
                    self.frames.pop();
                }
                Builder::Annotation => {
                    self.annotated_start = Some(frame.start);
                    self.frames.pop();
                    return Ok(None);
                }
                Builder::Record { .. } | Builder::Sequence(_) | Builder::Set(_) | Builder::Dictionary { .. } => {
                    self.add_child(value, (start, self.cursor.position()))?;
                    return Ok(None);
                }
            }
        }
    }

    fn add_child(&mut self, value: crate::model::PackedValue, span: Span) -> Result<(), Failure> {
        let input = self.cursor.bytes();
        let policy = self.policy;
        let limit = self.limits.max_container_items;
        let Some(frame) = self.frames.last_mut() else {
            return Err(Failure::UnexpectedEnd);
        };
        frame.children = frame.children.saturating_add(1);
        if frame.children > limit {
            return Err(Failure::ContainerExceeded { limit });
        }
        match &mut frame.builder {
            Builder::Record { label, fields } => match label {
                Some(_) => fields.push(value),
                None => *label = Some(value),
            },
            Builder::Sequence(items) => items.push(value),
            Builder::Set(set) => {
                let encoded = canonical_key(input, span, &value, policy);
                require_ascending(set.encoded().next_back(), &encoded)?;
                if !set.insert_encoded(encoded, value) {
                    return Err(Failure::DuplicateElement);
                }
            }
            Builder::Dictionary { dictionary, key } => match key.take() {
                None => {
                    let encoded = canonical_key(input, span, &value, policy);
                    require_ascending(dictionary.encoded_entries().next_back().map(|(last, _)| last), &encoded)?;
                    *key = Some((encoded, value));
                }
                Some((encoded, key_value)) => {
                    if !dictionary.insert_encoded(encoded, key_value, value) {
                        return Err(Failure::DuplicateElement);
                    }
                }
            },
            Builder::Embedded | Builder::Annotation => return Err(Failure::MissingValue),
        }
        Ok(())
    }
}

/// Set elements and dictionary keys must appear in strictly increasing order of their canonical
/// (annotation-free) bytes.
fn require_ascending(last: Option<&[u8]>, current: &[u8]) -> Result<(), Failure> {
    let Some(last) = last else {
        return Ok(());
    };
    match last.cmp(current) {
        core::cmp::Ordering::Less => Ok(()),
        core::cmp::Ordering::Equal => Err(Failure::DuplicateElement),
        core::cmp::Ordering::Greater => Err(Failure::UnsortedElements),
    }
}

/// The canonical bytes of a decoded element: its own input span when annotations are rejected,
/// otherwise its re-encoding without the skipped annotations.
fn canonical_key(
    input: &[u8],
    span: Span,
    value: &crate::model::PackedValue,
    policy: super::AnnotationPolicy,
) -> alloc::vec::Vec<u8> {
    match policy {
        super::AnnotationPolicy::Reject => input[span.0..span.1].to_vec(),
        super::AnnotationPolicy::Skip => crate::writer::encode_canonical(value),
    }
}
