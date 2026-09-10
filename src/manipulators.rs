use jsonptr::{resolve::Error as ResolveError, Pointer};
use serde_json::{Map, Value};

use crate::types::discard;
use crate::types::{JqesqueError, Operation, Path, PathErrorKind, PathToken};

/// Target-dependent growth, before any mutation. Missing or replaced containers start empty.
pub(crate) fn path_array_growth(document: &Value, path: &Path) -> usize {
    let tokens = path.tokens();
    let mut current = Some(document);
    let mut growth = 0usize;
    for token in tokens {
        current = match token {
            PathToken::Key(key) => current
                .and_then(|value| value.as_object())
                .and_then(|map| map.get(key)),
            PathToken::Index(index) => {
                let array = current.and_then(|value| value.as_array());
                growth =
                    growth.saturating_add((index + 1).saturating_sub(array.map_or(0, Vec::len)));
                array.and_then(|array| array.get(*index))
            }
        };
    }
    growth
}

pub(crate) fn apply_at_path(
    document: &mut Value,
    path: &Path,
    value: &Value,
    operation: Operation,
) {
    apply_at_tokens(document, path.tokens(), value, operation);
}

fn apply_at_tokens(
    document: &mut Value,
    tokens: &[PathToken],
    value: &Value,
    operation: Operation,
) {
    let Some((token, remaining)) = tokens.split_first() else {
        match operation {
            Operation::Merge => merge_json(document, value),
            Operation::MergePatch => merge_patch(document, value),
            _ => replace_value(document, value.clone()),
        }
        return;
    };
    let child = match token {
        PathToken::Key(key) => {
            if !document.is_object() {
                replace_value(document, Value::Object(Map::new()));
            }
            document
                .as_object_mut()
                .expect("object established above")
                .entry(key.clone())
                .or_insert(Value::Null)
        }
        PathToken::Index(index) => {
            if !document.is_array() {
                replace_value(document, Value::Array(Vec::new()));
            }
            let array = document.as_array_mut().expect("array established above");
            if *index >= array.len() {
                array.resize(index + 1, Value::Null);
            }
            &mut array[*index]
        }
    };
    apply_at_tokens(child, remaining, value, operation);
}

fn merge_json(target: &mut Value, patch: &Value) {
    match (target, patch) {
        (Value::Object(target), Value::Object(patch)) => {
            for (key, value) in patch {
                merge_json(target.entry(key.clone()).or_insert(Value::Null), value);
            }
        }
        (Value::Array(target), Value::Array(patch)) => {
            for (index, value) in patch.iter().enumerate() {
                if let Some(target) = target.get_mut(index) {
                    merge_json(target, value);
                } else {
                    target.push(value.clone());
                }
            }
        }
        (target, patch) => replace_value(target, patch.clone()),
    }
}

fn replace_value(target: &mut Value, value: Value) {
    let old = std::mem::replace(target, value);
    discard(old);
}

fn merge_patch(target: &mut Value, patch: &Value) {
    if let Value::Object(patch) = patch {
        if !target.is_object() {
            replace_value(target, Value::Object(Map::new()));
        }
        let target = target.as_object_mut().expect("object established above");
        for (key, value) in patch {
            if value.is_null() {
                if let Some(old) = target.remove(key) {
                    discard(old);
                }
            } else {
                merge_patch(target.entry(key.clone()).or_insert(Value::Null), value);
            }
        }
    } else {
        replace_value(target, patch.clone());
    }
}

/// Holds the exclusive document borrow and resolved target until budget approval and mutation.
/// No caller can change the container or invalidate an index between preparation and application.
pub(crate) struct PreparedPatch<'a> {
    target: PatchTarget<'a>,
    slots: usize,
}
enum PatchTarget<'a> {
    Replace(&'a mut Value),
    AddObject(&'a mut Map<String, Value>, String),
    AddArray(&'a mut Vec<Value>, usize),
    RemoveObject(&'a mut Map<String, Value>, String),
    RemoveArray(&'a mut Vec<Value>, usize),
}
impl PreparedPatch<'_> {
    pub(crate) fn array_slots(&self) -> usize {
        self.slots
    }
    pub(crate) fn apply(self, value: &Value) {
        match self.target {
            PatchTarget::Replace(target) => replace_value(target, value.clone()),
            PatchTarget::AddObject(target, key) => {
                if let Some(old) = target.insert(key, value.clone()) {
                    discard(old);
                }
            }
            PatchTarget::AddArray(target, index) => target.insert(index, value.clone()),
            PatchTarget::RemoveObject(target, key) => {
                if let Some(old) = target.remove(&key) {
                    discard(old);
                }
            }
            PatchTarget::RemoveArray(target, index) => discard(target.remove(index)),
        }
    }
}

pub(crate) fn prepare_patch<'a>(
    document: &'a mut Value,
    path: &Path,
    operation: Operation,
) -> Result<PreparedPatch<'a>, JqesqueError> {
    let pointer = path.pointer();
    if operation == Operation::Replace || (operation == Operation::Add && path.is_root()) {
        let target = pointer
            .resolve_mut(document)
            .map_err(|error| map_resolve_error(path, error))?;
        return Ok(PreparedPatch {
            target: PatchTarget::Replace(target),
            slots: 0,
        });
    }
    let Some((parent, last)) = pointer.split_back() else {
        return Err(path_error(path, PathErrorKind::RootRemoval, 0));
    };
    let parent = parent
        .resolve_mut(document)
        .map_err(|error| map_resolve_error(path, error))?;
    let position = path.tokens().len() - 1;
    let (target, slots) = match parent {
        Value::Object(target) => {
            let key = last.decoded().into_owned();
            if operation == Operation::Add {
                (PatchTarget::AddObject(target, key), 0)
            } else if target.contains_key(&key) {
                (PatchTarget::RemoveObject(target, key), 0)
            } else {
                return Err(path_error(path, PathErrorKind::MissingPath, position));
            }
        }
        Value::Array(target) => {
            let index = last
                .to_index()
                .map_err(|_| path_error(path, PathErrorKind::InvalidIndex, position))?;
            let index = if operation == Operation::Add {
                index.for_len_incl(target.len())
            } else {
                index.for_len(target.len())
            }
            .map_err(|_| path_error(path, PathErrorKind::IndexOutOfBounds, position))?;
            if operation == Operation::Add {
                (PatchTarget::AddArray(target, index), 1)
            } else {
                (PatchTarget::RemoveArray(target, index), 0)
            }
        }
        _ => return Err(path_error(path, PathErrorKind::TypeConflict, position)),
    };
    Ok(PreparedPatch { target, slots })
}

fn map_resolve_error(path: &Path, error: ResolveError) -> JqesqueError {
    let kind = match error {
        ResolveError::FailedToParseIndex { .. } => PathErrorKind::InvalidIndex,
        ResolveError::OutOfBounds { .. } => PathErrorKind::IndexOutOfBounds,
        ResolveError::NotFound { .. } => PathErrorKind::MissingPath,
        ResolveError::Unreachable { .. } => PathErrorKind::TypeConflict,
    };
    path_error(path, kind, error.position())
}

fn path_error(path: &Path, kind: PathErrorKind, token: usize) -> JqesqueError {
    JqesqueError::PathError {
        kind,
        path: path.to_json_pointer(),
        token,
    }
}

fn resolve_pointer<'a>(
    document: &'a Value,
    pointer: &Pointer,
    path: &Path,
) -> Result<&'a Value, JqesqueError> {
    pointer
        .resolve(document)
        .map_err(|error| map_resolve_error(path, error))
}

pub(crate) fn resolve<'a>(document: &'a Value, path: &Path) -> Result<&'a Value, JqesqueError> {
    resolve_pointer(document, &path.pointer(), path)
}

/// Preflight classifies dependency errors at the boundary and returns new array slots for Add.
pub(crate) fn check_patch_path(
    document: &Value,
    path: &Path,
    operation: Operation,
) -> Result<usize, JqesqueError> {
    if operation != Operation::Add {
        if operation == Operation::Remove && path.is_root() {
            return Err(path_error(path, PathErrorKind::RootRemoval, 0));
        }
        resolve(document, path)?;
        return Ok(0);
    }
    let pointer = path.pointer();
    let Some((parent, last)) = pointer.split_back() else {
        return Ok(0);
    };
    let parent = resolve_pointer(document, parent, path)?;
    let position = path.tokens().len() - 1;
    match parent {
        Value::Object(_) => Ok(0),
        Value::Array(array) => {
            let index = last
                .to_index()
                .map_err(|_| path_error(path, PathErrorKind::InvalidIndex, position))?;
            index
                .for_len_incl(array.len())
                .map_err(|_| path_error(path, PathErrorKind::IndexOutOfBounds, position))?;
            Ok(1)
        }
        _ => Err(path_error(path, PathErrorKind::TypeConflict, position)),
    }
}

mod numbers;
use numbers::number_eq;

pub(crate) fn numeric_json_eq(a: &Value, b: &Value) -> bool {
    // Iterative traversal also handles deeply nested caller-owned target documents.
    let mut pending = vec![(a, b)];
    while let Some((a, b)) = pending.pop() {
        match (a, b) {
            (Value::Number(a), Value::Number(b)) if number_eq(a, b) => {}
            (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
                pending.extend(a.iter().zip(b))
            }
            (Value::Object(a), Value::Object(b)) if a.len() == b.len() => {
                for (key, value) in a {
                    let Some(other) = b.get(key) else {
                        return false;
                    };
                    pending.push((value, other));
                }
            }
            (Value::Null, Value::Null) => {}
            (Value::Bool(a), Value::Bool(b)) if a == b => {}
            (Value::String(a), Value::String(b)) if a == b => {}
            _ => return false,
        }
    }
    true
}
