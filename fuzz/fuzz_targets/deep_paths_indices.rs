#![no_main]

use jqesque::{Jqesque, ParseOptions, Separator};
use libfuzzer_sys::fuzz_target;
use serde_json::json;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() {
        return;
    }

    let depth = (data[0] as usize % 300) + 1;
    let index = data.iter().skip(1).fold(0usize, |acc, b| {
        acc.saturating_mul(10).saturating_add((b % 10) as usize)
    });

    let path = std::iter::repeat_n("a", depth)
        .collect::<Vec<_>>()
        .join(".");
    let input = format!(">{}[{}]=1", path, index);

    for separator in [Separator::Dot, Separator::Slash, Separator::Custom(':')] {
        let options = ParseOptions::new(separator)
            .strict_json_values(false)
            .max_path_depth(64)
            .max_array_index(50_000);

        if let Ok(parsed) = Jqesque::from_str_with_options(&input, options) {
            let mut json_obj = json!({
                "seed": [1, 2, 3],
                "settings": { "theme": "light" }
            });
            let _ = parsed.apply_to(&mut json_obj);
        }
    }
});
