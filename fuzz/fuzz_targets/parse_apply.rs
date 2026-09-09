#![no_main]

use jqesque::{Jqesque, JqesqueError, ParseOptions, Separator};
use libfuzzer_sys::fuzz_target;
use serde_json::json;

fuzz_target!(|data: &[u8]| {
    if data.len() > 4096 {
        return;
    }
    let input = String::from_utf8_lossy(data);
    let depth_limit = data.first().copied().unwrap_or_default() as usize % 65;

    for separator in [Separator::Dot, Separator::Slash, Separator::Custom(':')] {
        let options = ParseOptions::new(separator)
            .strict_json_values(data.len().is_multiple_of(2))
            .max_path_depth(depth_limit)
            .max_array_index(256);

        match Jqesque::from_str_with_options(&input, options) {
            Ok(parsed) => {
                assert!(parsed.tokens().len() <= depth_limit);
                let mut json_obj = json!({
                    "seed": [1, 2, 3],
                    "settings": { "theme": "light" }
                });

                let _ = parsed.as_json();
                let _ = parsed.apply_to(&mut json_obj);
            }
            Err(JqesqueError::LimitExceededError {
                kind: "path depth",
                limit,
                found,
            }) => {
                assert_eq!(limit, depth_limit);
                assert_eq!(found, limit + 1);
            }
            Err(_) => {}
        }
    }
});
