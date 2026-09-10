use super::Separator;

pub const DEFAULT_MAX_PATH_DEPTH: usize = 128;
pub const DEFAULT_MAX_ARRAY_INDEX: usize = 1_000_000;
pub const DEFAULT_MAX_ARRAY_SLOTS: usize = DEFAULT_MAX_ARRAY_INDEX + 1;
pub const DEFAULT_MAX_INPUT_BYTES: usize = 1_048_576;
pub const DEFAULT_MAX_VALUE_DEPTH: usize = 128;
pub const DEFAULT_MAX_VALUE_NODES: usize = 1_000_000;
pub const DEFAULT_MAX_BATCH_LENGTH: usize = 1_024;

/// Parsing budgets. Setters can tighten the exported global ceilings, never raise them.
#[derive(Debug, Clone, Copy)]
pub struct ParseOptions {
    separator: Separator,
    strict_json_values: bool,
    max_path_depth: usize,
    max_array_index: usize,
    max_input_bytes: usize,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self::new(Separator::Dot)
    }
}

impl ParseOptions {
    pub fn new(separator: Separator) -> Self {
        Self {
            separator,
            strict_json_values: false,
            max_path_depth: DEFAULT_MAX_PATH_DEPTH,
            max_array_index: DEFAULT_MAX_ARRAY_INDEX,
            max_input_bytes: DEFAULT_MAX_INPUT_BYTES,
        }
    }

    pub fn strict_json_values(mut self, enabled: bool) -> Self {
        self.strict_json_values = enabled;
        self
    }

    /// Counts each key and bracketed index as one token; checked before decoding the value.
    pub fn max_path_depth(mut self, limit: usize) -> Self {
        self.max_path_depth = limit.min(DEFAULT_MAX_PATH_DEPTH);
        self
    }

    /// Checked during path parsing, before decoding the value.
    pub fn max_array_index(mut self, limit: usize) -> Self {
        self.max_array_index = limit.min(DEFAULT_MAX_ARRAY_INDEX);
        self
    }

    /// Checked before tokenization or value decoding.
    pub fn max_input_bytes(mut self, limit: usize) -> Self {
        self.max_input_bytes = limit.min(DEFAULT_MAX_INPUT_BYTES);
        self
    }

    pub fn separator(&self) -> Separator {
        self.separator
    }
    pub fn strict_json_values_enabled(&self) -> bool {
        self.strict_json_values
    }
    pub fn max_path_depth_limit(&self) -> usize {
        self.max_path_depth
    }
    pub fn max_array_index_limit(&self) -> usize {
        self.max_array_index
    }
    pub fn max_input_bytes_limit(&self) -> usize {
        self.max_input_bytes
    }
}

/// Budget for one application, or cumulatively for a batch.
/// Counts newly created path array slots plus all array slots in copied payloads.
#[derive(Debug, Clone, Copy)]
pub struct ApplyOptions {
    max_array_slots: usize,
}

impl Default for ApplyOptions {
    fn default() -> Self {
        Self {
            max_array_slots: DEFAULT_MAX_ARRAY_SLOTS,
        }
    }
}

impl ApplyOptions {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn max_array_slots(mut self, limit: usize) -> Self {
        self.max_array_slots = limit.min(DEFAULT_MAX_ARRAY_SLOTS);
        self
    }
    pub fn max_array_slots_limit(&self) -> usize {
        self.max_array_slots
    }
}
