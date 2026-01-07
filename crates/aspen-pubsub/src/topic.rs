//! Topic and TopicPattern types for pub/sub.
//!
//! Topics are hierarchical names separated by dots (e.g., "orders.created").
//! Patterns support NATS-style wildcards:
//! - `*` matches exactly one segment
//! - `>` matches zero or more segments (must be last)

use serde::Deserialize;
use serde::Serialize;

use crate::constants::MAX_SEGMENT_LENGTH;
use crate::constants::MAX_TOPIC_SEGMENTS;
use crate::constants::TOPIC_SEGMENT_SEPARATOR;
use crate::constants::WILDCARD_MULTI;
use crate::constants::WILDCARD_SINGLE;
use crate::error::PatternInvalidSnafu;
use crate::error::Result;
use crate::error::TopicEmptySnafu;
use crate::error::TopicInvalidCharacterSnafu;
use crate::error::TopicSegmentEmptySnafu;
use crate::error::TopicSegmentTooLongSnafu;
use crate::error::TopicTooManySegmentsSnafu;

/// A topic name for pub/sub events.
///
/// Topics are hierarchical names separated by dots. For example:
/// - `"orders"` - single segment
/// - `"orders.created"` - two segments
/// - `"orders.us.east.created"` - four segments
///
/// Topic names must:
/// - Be non-empty
/// - Have at most `MAX_TOPIC_SEGMENTS` segments
/// - Have segments at most `MAX_SEGMENT_LENGTH` bytes each
/// - Contain only alphanumeric characters, hyphens, and underscores
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Topic(String);

impl Topic {
    /// Create a new topic from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the topic name is invalid (empty, too long, etc.).
    ///
    /// # Examples
    ///
    /// ```
    /// use aspen_pubsub::Topic;
    ///
    /// let topic = Topic::new("orders.created").unwrap();
    /// assert_eq!(topic.as_str(), "orders.created");
    /// ```
    pub fn new(name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        Self::validate(&name)?;
        Ok(Self(name))
    }

    /// Get the topic name as a string slice.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Get the segments of the topic.
    ///
    /// # Examples
    ///
    /// ```
    /// use aspen_pubsub::Topic;
    ///
    /// let topic = Topic::new("orders.us.created").unwrap();
    /// let segments: Vec<_> = topic.segments().collect();
    /// assert_eq!(segments, vec!["orders", "us", "created"]);
    /// ```
    pub fn segments(&self) -> impl Iterator<Item = &str> {
        self.0.split(TOPIC_SEGMENT_SEPARATOR)
    }

    /// Get the number of segments in the topic.
    pub fn segment_count(&self) -> usize {
        self.0.split(TOPIC_SEGMENT_SEPARATOR).count()
    }

    /// Validate a topic name.
    fn validate(name: &str) -> Result<()> {
        if name.is_empty() {
            return TopicEmptySnafu.fail();
        }

        let segments: Vec<&str> = name.split(TOPIC_SEGMENT_SEPARATOR).collect();

        if segments.len() > MAX_TOPIC_SEGMENTS {
            return TopicTooManySegmentsSnafu {
                segments: segments.len(),
            }
            .fail();
        }

        for (position, segment) in segments.iter().enumerate() {
            if segment.is_empty() {
                return TopicSegmentEmptySnafu { position }.fail();
            }

            if segment.len() > MAX_SEGMENT_LENGTH {
                return TopicSegmentTooLongSnafu {
                    segment: (*segment).to_string(),
                    length: segment.len(),
                }
                .fail();
            }

            // Validate characters: alphanumeric, hyphen, underscore
            for ch in segment.chars() {
                if !ch.is_alphanumeric() && ch != '-' && ch != '_' {
                    return TopicInvalidCharacterSnafu {
                        segment: (*segment).to_string(),
                        character: ch,
                    }
                    .fail();
                }
            }
        }

        Ok(())
    }
}

impl std::fmt::Display for Topic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl AsRef<str> for Topic {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// A segment in a topic pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PatternSegment {
    /// Matches this exact literal string.
    Literal(String),
    /// Matches exactly one segment (the `*` wildcard).
    Single,
    /// Matches zero or more segments (the `>` wildcard, must be last).
    Multi,
}

/// A pattern for matching topics.
///
/// Patterns support NATS-style wildcards:
/// - `*` matches exactly one segment: `"orders.*"` matches `"orders.created"` but not
///   `"orders.us.created"`
/// - `>` matches zero or more segments: `"orders.>"` matches `"orders"`, `"orders.created"`, and
///   `"orders.us.created"`
///
/// The `>` wildcard must be the last segment in a pattern.
///
/// # Examples
///
/// ```
/// use aspen_pubsub::{Topic, TopicPattern};
///
/// let pattern = TopicPattern::new("orders.*").unwrap();
/// assert!(pattern.matches(&Topic::new("orders.created").unwrap()));
/// assert!(!pattern.matches(&Topic::new("orders.us.created").unwrap()));
///
/// let pattern = TopicPattern::new("orders.>").unwrap();
/// assert!(pattern.matches(&Topic::new("orders").unwrap()));
/// assert!(pattern.matches(&Topic::new("orders.created").unwrap()));
/// assert!(pattern.matches(&Topic::new("orders.us.created").unwrap()));
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TopicPattern {
    /// The pattern segments.
    segments: Vec<PatternSegment>,
    /// Original pattern string for display.
    original: String,
}

impl TopicPattern {
    /// Create a new topic pattern from a string.
    ///
    /// # Errors
    ///
    /// Returns an error if the pattern is invalid (e.g., `>` not at end).
    pub fn new(pattern: impl Into<String>) -> Result<Self> {
        let pattern = pattern.into();
        let segments = Self::parse(&pattern)?;
        Ok(Self {
            segments,
            original: pattern,
        })
    }

    /// Create a pattern that matches a single exact topic.
    pub fn exact(topic: &Topic) -> Self {
        let segments = topic.segments().map(|s| PatternSegment::Literal(s.to_string())).collect();
        Self {
            segments,
            original: topic.as_str().to_string(),
        }
    }

    /// Create a pattern that matches all topics.
    pub fn all() -> Self {
        Self {
            segments: vec![PatternSegment::Multi],
            original: WILDCARD_MULTI.to_string(),
        }
    }

    /// Check if this pattern matches a topic.
    pub fn matches(&self, topic: &Topic) -> bool {
        let topic_segments: Vec<&str> = topic.segments().collect();
        self.matches_segments(&topic_segments, 0, 0)
    }

    /// Get the literal prefix of this pattern (segments before the first wildcard).
    ///
    /// This is used for server-side prefix filtering.
    ///
    /// # Examples
    ///
    /// ```
    /// use aspen_pubsub::TopicPattern;
    ///
    /// let pattern = TopicPattern::new("orders.us.*").unwrap();
    /// assert_eq!(pattern.literal_prefix(), vec!["orders", "us"]);
    ///
    /// let pattern = TopicPattern::new("*.created").unwrap();
    /// assert!(pattern.literal_prefix().is_empty());
    /// ```
    pub fn literal_prefix(&self) -> Vec<&str> {
        self.segments
            .iter()
            .take_while(|s| matches!(s, PatternSegment::Literal(_)))
            .filter_map(|s| match s {
                PatternSegment::Literal(lit) => Some(lit.as_str()),
                _ => None,
            })
            .collect()
    }

    /// Check if this pattern has any wildcards.
    pub fn has_wildcards(&self) -> bool {
        self.segments.iter().any(|s| matches!(s, PatternSegment::Single | PatternSegment::Multi))
    }

    /// Get the original pattern string.
    pub fn as_str(&self) -> &str {
        &self.original
    }

    /// Parse a pattern string into segments.
    fn parse(pattern: &str) -> Result<Vec<PatternSegment>> {
        if pattern.is_empty() {
            return PatternInvalidSnafu {
                reason: "pattern cannot be empty".to_string(),
            }
            .fail();
        }

        let parts: Vec<&str> = pattern.split(TOPIC_SEGMENT_SEPARATOR).collect();

        if parts.len() > MAX_TOPIC_SEGMENTS {
            return PatternInvalidSnafu {
                reason: format!("pattern has {} segments, maximum is {}", parts.len(), MAX_TOPIC_SEGMENTS),
            }
            .fail();
        }

        let mut segments = Vec::with_capacity(parts.len());
        let mut found_multi = false;

        for (i, part) in parts.iter().enumerate() {
            if found_multi {
                return PatternInvalidSnafu {
                    reason: format!("'{WILDCARD_MULTI}' wildcard must be the last segment, found '{part}' after it"),
                }
                .fail();
            }

            if part.is_empty() {
                return PatternInvalidSnafu {
                    reason: format!("empty segment at position {i}"),
                }
                .fail();
            }

            let segment = if *part == WILDCARD_SINGLE {
                PatternSegment::Single
            } else if *part == WILDCARD_MULTI {
                found_multi = true;
                PatternSegment::Multi
            } else {
                // Validate as literal segment
                if part.len() > MAX_SEGMENT_LENGTH {
                    return PatternInvalidSnafu {
                        reason: format!("segment '{part}' exceeds maximum length of {MAX_SEGMENT_LENGTH}"),
                    }
                    .fail();
                }

                // Check for partial wildcards (not allowed)
                if part.contains('*') || part.contains('>') {
                    return PatternInvalidSnafu {
                        reason: format!("wildcards must be standalone segments, found '{part}'"),
                    }
                    .fail();
                }

                PatternSegment::Literal((*part).to_string())
            };

            segments.push(segment);
        }

        Ok(segments)
    }

    /// Recursive pattern matching.
    fn matches_segments(&self, topic_segments: &[&str], pattern_idx: usize, topic_idx: usize) -> bool {
        // If we've consumed the entire pattern
        if pattern_idx >= self.segments.len() {
            // Match only if we've also consumed the entire topic
            return topic_idx >= topic_segments.len();
        }

        match &self.segments[pattern_idx] {
            PatternSegment::Literal(lit) => {
                // Must have a topic segment that matches exactly
                if topic_idx >= topic_segments.len() {
                    return false;
                }
                if topic_segments[topic_idx] != lit {
                    return false;
                }
                self.matches_segments(topic_segments, pattern_idx + 1, topic_idx + 1)
            }
            PatternSegment::Single => {
                // Must have exactly one topic segment (any value)
                if topic_idx >= topic_segments.len() {
                    return false;
                }
                self.matches_segments(topic_segments, pattern_idx + 1, topic_idx + 1)
            }
            PatternSegment::Multi => {
                // Match zero or more remaining segments
                // Since Multi must be last, we match everything remaining
                true
            }
        }
    }
}

impl std::fmt::Display for TopicPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.original)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::PubSubError;

    // ========================================================================
    // Topic Tests
    // ========================================================================

    #[test]
    fn test_topic_valid() {
        assert!(Topic::new("orders").is_ok());
        assert!(Topic::new("orders.created").is_ok());
        assert!(Topic::new("orders.us.east.created").is_ok());
        assert!(Topic::new("my-topic").is_ok());
        assert!(Topic::new("my_topic").is_ok());
        assert!(Topic::new("topic123").is_ok());
    }

    #[test]
    fn test_topic_empty() {
        assert!(matches!(Topic::new(""), Err(PubSubError::TopicEmpty)));
    }

    #[test]
    fn test_topic_empty_segment() {
        assert!(matches!(Topic::new("orders..created"), Err(PubSubError::TopicSegmentEmpty { position: 1 })));
        assert!(matches!(Topic::new(".orders"), Err(PubSubError::TopicSegmentEmpty { position: 0 })));
        assert!(matches!(Topic::new("orders."), Err(PubSubError::TopicSegmentEmpty { .. })));
    }

    #[test]
    fn test_topic_too_many_segments() {
        let long_topic = (0..20).map(|i| format!("seg{i}")).collect::<Vec<_>>().join(".");
        assert!(matches!(Topic::new(long_topic), Err(PubSubError::TopicTooManySegments { segments: 20 })));
    }

    #[test]
    fn test_topic_segment_too_long() {
        let long_segment = "x".repeat(300);
        assert!(matches!(Topic::new(long_segment), Err(PubSubError::TopicSegmentTooLong { .. })));
    }

    #[test]
    fn test_topic_invalid_characters() {
        assert!(matches!(
            Topic::new("orders.cre ated"),
            Err(PubSubError::TopicInvalidCharacter { character: ' ', .. })
        ));
        assert!(matches!(
            Topic::new("orders/created"),
            Err(PubSubError::TopicInvalidCharacter { character: '/', .. })
        ));
    }

    #[test]
    fn test_topic_segments() {
        let topic = Topic::new("orders.us.created").unwrap();
        let segments: Vec<_> = topic.segments().collect();
        assert_eq!(segments, vec!["orders", "us", "created"]);
        assert_eq!(topic.segment_count(), 3);
    }

    // ========================================================================
    // Pattern Tests
    // ========================================================================

    #[test]
    fn test_pattern_literal() {
        let pattern = TopicPattern::new("orders.created").unwrap();
        assert!(pattern.matches(&Topic::new("orders.created").unwrap()));
        assert!(!pattern.matches(&Topic::new("orders.updated").unwrap()));
        assert!(!pattern.matches(&Topic::new("orders").unwrap()));
        assert!(!pattern.matches(&Topic::new("orders.created.extra").unwrap()));
    }

    #[test]
    fn test_pattern_single_wildcard() {
        let pattern = TopicPattern::new("orders.*").unwrap();
        assert!(pattern.matches(&Topic::new("orders.created").unwrap()));
        assert!(pattern.matches(&Topic::new("orders.updated").unwrap()));
        assert!(!pattern.matches(&Topic::new("orders").unwrap())); // * requires exactly one
        assert!(!pattern.matches(&Topic::new("orders.us.created").unwrap())); // * matches exactly one

        let pattern = TopicPattern::new("*.created").unwrap();
        assert!(pattern.matches(&Topic::new("orders.created").unwrap()));
        assert!(pattern.matches(&Topic::new("users.created").unwrap()));
        assert!(!pattern.matches(&Topic::new("orders.us.created").unwrap()));

        let pattern = TopicPattern::new("orders.*.created").unwrap();
        assert!(pattern.matches(&Topic::new("orders.us.created").unwrap()));
        assert!(pattern.matches(&Topic::new("orders.emea.created").unwrap()));
        assert!(!pattern.matches(&Topic::new("orders.created").unwrap()));
        assert!(!pattern.matches(&Topic::new("orders.us.east.created").unwrap()));
    }

    #[test]
    fn test_pattern_multi_wildcard() {
        let pattern = TopicPattern::new("orders.>").unwrap();
        assert!(pattern.matches(&Topic::new("orders").unwrap())); // > matches zero
        assert!(pattern.matches(&Topic::new("orders.created").unwrap()));
        assert!(pattern.matches(&Topic::new("orders.us.created").unwrap()));
        assert!(pattern.matches(&Topic::new("orders.us.east.created").unwrap()));
        assert!(!pattern.matches(&Topic::new("users.created").unwrap()));

        let pattern = TopicPattern::new(">").unwrap();
        assert!(pattern.matches(&Topic::new("anything").unwrap()));
        assert!(pattern.matches(&Topic::new("orders.us.created").unwrap()));
    }

    #[test]
    fn test_pattern_combined_wildcards() {
        let pattern = TopicPattern::new("orders.*.>").unwrap();
        assert!(pattern.matches(&Topic::new("orders.us").unwrap()));
        assert!(pattern.matches(&Topic::new("orders.us.created").unwrap()));
        assert!(pattern.matches(&Topic::new("orders.emea.east.created").unwrap()));
        assert!(!pattern.matches(&Topic::new("orders").unwrap())); // needs at least one for *
    }

    #[test]
    fn test_pattern_invalid_multi_not_last() {
        assert!(matches!(TopicPattern::new("orders.>.created"), Err(PubSubError::PatternInvalid { .. })));
    }

    #[test]
    fn test_pattern_invalid_partial_wildcard() {
        assert!(matches!(TopicPattern::new("orders.cre*ted"), Err(PubSubError::PatternInvalid { .. })));
        assert!(matches!(TopicPattern::new("orders.foo>"), Err(PubSubError::PatternInvalid { .. })));
    }

    #[test]
    fn test_pattern_literal_prefix() {
        let pattern = TopicPattern::new("orders.us.>").unwrap();
        assert_eq!(pattern.literal_prefix(), vec!["orders", "us"]);

        let pattern = TopicPattern::new("orders.*").unwrap();
        assert_eq!(pattern.literal_prefix(), vec!["orders"]);

        let pattern = TopicPattern::new("*.created").unwrap();
        assert!(pattern.literal_prefix().is_empty());

        let pattern = TopicPattern::new("orders.us.created").unwrap();
        assert_eq!(pattern.literal_prefix(), vec!["orders", "us", "created"]);
    }

    #[test]
    fn test_pattern_has_wildcards() {
        assert!(!TopicPattern::new("orders.created").unwrap().has_wildcards());
        assert!(TopicPattern::new("orders.*").unwrap().has_wildcards());
        assert!(TopicPattern::new("orders.>").unwrap().has_wildcards());
    }

    #[test]
    fn test_pattern_exact() {
        let topic = Topic::new("orders.created").unwrap();
        let pattern = TopicPattern::exact(&topic);
        assert!(pattern.matches(&topic));
        assert!(!pattern.has_wildcards());
    }

    #[test]
    fn test_pattern_all() {
        let pattern = TopicPattern::all();
        assert!(pattern.matches(&Topic::new("anything").unwrap()));
        assert!(pattern.matches(&Topic::new("orders.us.created").unwrap()));
    }
}
