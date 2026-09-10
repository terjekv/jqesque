use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use thiserror::Error;

use super::serde_support::bounded_sequence;
use super::value::check_clone;
use super::{
    check_limit, ApplyBudget, ApplyOptions, Jqesque, JqesqueError, LimitKind, Operation,
    ParseOptions, DEFAULT_MAX_BATCH_LENGTH, DEFAULT_MAX_INPUT_BYTES, DEFAULT_MAX_VALUE_NODES,
};

/// Ordered assignments with bounded length. Serializes as an array of assignments.
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(transparent)]
pub struct Batch {
    assignments: Vec<Jqesque>,
}

/// Failure in a batch; `index` is zero-based. The source preserves the original error kind.
#[derive(Debug, Error, PartialEq)]
#[error("assignment {index} failed: {source}")]
pub struct BatchError {
    pub index: usize,
    #[source]
    pub source: JqesqueError,
}

impl Batch {
    pub fn new(assignments: Vec<Jqesque>) -> Result<Self, JqesqueError> {
        check_limit(
            LimitKind::BatchLength,
            assignments.len(),
            DEFAULT_MAX_BATCH_LENGTH,
        )?;
        let mut nodes = 0usize;
        let mut bytes = 0usize;
        for assignment in &assignments {
            nodes = nodes.saturating_add(assignment.payload_nodes());
            bytes = bytes.saturating_add(assignment.payload_bytes());
            check_limit(LimitKind::ValueBytes, bytes, DEFAULT_MAX_INPUT_BYTES)?;
            check_limit(LimitKind::ValueNodes, nodes, DEFAULT_MAX_VALUE_NODES)?;
        }
        Ok(Self { assignments })
    }

    /// Parse all assignments before any application. Stops at the first error or excess item.
    pub fn parse<I, S>(inputs: I, options: ParseOptions) -> Result<Self, BatchError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut assignments = Vec::new();
        let mut bytes = 0usize;
        let mut nodes = 0usize;
        for (index, input) in inputs.into_iter().enumerate() {
            check_limit(LimitKind::BatchLength, index + 1, DEFAULT_MAX_BATCH_LENGTH)
                .map_err(|source| BatchError { index, source })?;
            bytes = bytes.saturating_add(input.as_ref().len());
            check_limit(LimitKind::InputBytes, bytes, DEFAULT_MAX_INPUT_BYTES)
                .map_err(|source| BatchError { index, source })?;
            let assignment = Jqesque::from_str_with_options(input.as_ref(), options)
                .map_err(|source| BatchError { index, source })?;
            nodes = nodes.saturating_add(assignment.payload_nodes());
            check_limit(LimitKind::ValueNodes, nodes, DEFAULT_MAX_VALUE_NODES)
                .map_err(|source| BatchError { index, source })?;
            assignments.push(assignment);
        }
        Ok(Self { assignments })
    }

    pub fn assignments(&self) -> &[Jqesque] {
        &self.assignments
    }

    /// Apply in order, keeping successful earlier changes if a later assignment fails.
    /// The allocation budget is shared across the entire batch.
    pub fn apply_to(
        &self,
        document: &mut Value,
        options: ApplyOptions,
    ) -> Result<Vec<Operation>, BatchError> {
        let mut budget = ApplyBudget::new(options);
        self.assignments
            .iter()
            .enumerate()
            .map(|(index, assignment)| {
                assignment
                    .apply_with_budget(document, &mut budget)
                    .map_err(|source| BatchError { index, source })
            })
            .collect()
    }

    /// Apply to a clone, committing only after every assignment succeeds.
    /// Cloning the caller's existing document is additional work outside the allocation budget.
    pub fn apply_atomically(
        &self,
        document: &mut Value,
        options: ApplyOptions,
    ) -> Result<Vec<Operation>, BatchError> {
        if self.assignments.is_empty() {
            return Ok(Vec::new());
        }
        check_clone(document).map_err(|source| BatchError { index: 0, source })?;
        let mut working = document.clone();
        let operations = self.apply_to(&mut working, options)?;
        *document = working;
        Ok(operations)
    }
}

impl<'de> Deserialize<'de> for Batch {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let mut nodes = 0usize;
        let mut bytes = 0usize;
        let assignments = bounded_sequence(
            deserializer,
            DEFAULT_MAX_BATCH_LENGTH,
            LimitKind::BatchLength,
            |assignment: &Jqesque| {
                nodes = nodes.saturating_add(assignment.payload_nodes());
                bytes = bytes.saturating_add(assignment.payload_bytes());
                check_limit(LimitKind::ValueBytes, bytes, DEFAULT_MAX_INPUT_BYTES)?;
                check_limit(LimitKind::ValueNodes, nodes, DEFAULT_MAX_VALUE_NODES)
            },
        )?;
        Ok(Self { assignments })
    }
}
