#![no_main]

use jqesque::{Jqesque, ParseOptions, Separator};
use libfuzzer_sys::fuzz_target;
use serde_json::json;

fuzz_target!(|data: &[u8]| {
    let input = String::from_utf8_lossy(data);

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

            let _ = parsed.as_json();
            let _ = parsed.apply_to(&mut json_obj);
        }
    }
});
