use std::fmt;

use jsonptr::{PointerBuf, Token};
use serde::{
    de::{DeserializeSeed, SeqAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};

use super::errors::check_limit;
use super::{
    JqesqueError, LimitKind, ParseOptions, DEFAULT_MAX_ARRAY_INDEX, DEFAULT_MAX_ARRAY_SLOTS,
    DEFAULT_MAX_INPUT_BYTES, DEFAULT_MAX_PATH_DEPTH,
};

/// Raw path component. Use `Path::new` to validate a complete path.
/// Numeric keys remain keys for Insert/Merge; JSON Patch interprets them using the target container.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PathToken {
    Key(String),
    Index(usize),
}

/// Validated reusable path. Its Serde representation is an array of `PathToken`s.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(transparent)]
pub struct Path {
    tokens: Vec<PathToken>,
    #[serde(skip)]
    key_bytes: usize,
}

impl Path {
    pub fn new(tokens: Vec<PathToken>) -> Result<Self, JqesqueError> {
        check_limit(LimitKind::PathDepth, tokens.len(), DEFAULT_MAX_PATH_DEPTH)?;
        let mut builder = PathBuilder::new(ParseOptions::default());
        for token in tokens {
            builder.push(token)?;
        }
        Ok(builder.finish())
    }

    /// The whole document, represented by an empty JSON Pointer.
    pub fn root() -> Self {
        Self {
            tokens: Vec::new(),
            key_bytes: 0,
        }
    }
    pub fn tokens(&self) -> &[PathToken] {
        &self.tokens
    }
    pub(super) fn key_bytes(&self) -> usize {
        self.key_bytes
    }

    pub fn is_root(&self) -> bool {
        self.tokens.is_empty()
    }

    /// RFC 6901 pointer, escaping each raw key exactly once.
    pub fn to_json_pointer(&self) -> String {
        self.pointer().to_string()
    }

    pub(crate) fn pointer(&self) -> PointerBuf {
        PointerBuf::from_tokens(self.tokens.iter().map(|token| match token {
            PathToken::Key(key) => Token::new(key.as_str()),
            PathToken::Index(index) => Token::new(index.to_string()),
        }))
    }
}

impl TryFrom<Vec<PathToken>> for Path {
    type Error = JqesqueError;
    fn try_from(tokens: Vec<PathToken>) -> Result<Self, Self::Error> {
        Self::new(tokens)
    }
}

impl<'de> Deserialize<'de> for Path {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct PathVisitor;
        impl<'de> Visitor<'de> for PathVisitor {
            type Value = Path;
            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a bounded sequence of path tokens")
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut access: A) -> Result<Path, A::Error> {
                let mut builder = PathBuilder::new(ParseOptions::default());
                while access.next_element_seed(NextToken(&mut builder))?.is_some() {}
                Ok(builder.finish())
            }
        }
        struct NextToken<'a>(&'a mut PathBuilder);
        impl<'de> DeserializeSeed<'de> for NextToken<'_> {
            type Value = ();
            fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<(), D::Error> {
                self.0.check_next().map_err(serde::de::Error::custom)?;
                let token = PathToken::deserialize(deserializer)?;
                self.0.push(token).map_err(serde::de::Error::custom)
            }
        }
        deserializer.deserialize_seq(PathVisitor)
    }
}

/// Accumulates validated tokens at the parser boundary; finish preserves that proof.
pub(crate) struct PathBuilder {
    tokens: Vec<PathToken>,
    options: ParseOptions,
    slots: usize,
    key_bytes: usize,
}

impl PathBuilder {
    pub(crate) fn new(options: ParseOptions) -> Self {
        Self {
            tokens: Vec::new(),
            options,
            slots: 0,
            key_bytes: 0,
        }
    }

    pub(crate) fn len(&self) -> usize {
        self.tokens.len()
    }

    pub(crate) fn check_next(&self) -> Result<(), JqesqueError> {
        check_limit(
            LimitKind::PathDepth,
            self.tokens.len() + 1,
            self.options.max_path_depth_limit(),
        )
    }

    pub(crate) fn push(&mut self, token: PathToken) -> Result<(), JqesqueError> {
        self.check_next()?;
        if let PathToken::Index(index) = &token {
            check_limit(
                LimitKind::ArrayIndex,
                *index,
                self.options
                    .max_array_index_limit()
                    .min(DEFAULT_MAX_ARRAY_INDEX),
            )?;
            let slots = self.slots.saturating_add(index.saturating_add(1));
            check_limit(LimitKind::ArraySlots, slots, DEFAULT_MAX_ARRAY_SLOTS)?;
            self.slots = slots;
        }
        if let PathToken::Key(key) = &token {
            let bytes = self.key_bytes.saturating_add(key.len());
            check_limit(LimitKind::PathBytes, bytes, DEFAULT_MAX_INPUT_BYTES)?;
            self.key_bytes = bytes;
        }
        self.tokens.push(token);
        Ok(())
    }

    pub(crate) fn finish(self) -> Path {
        Path {
            tokens: self.tokens,
            key_bytes: self.key_bytes,
        }
    }
}
