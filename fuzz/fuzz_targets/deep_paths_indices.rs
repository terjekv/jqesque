#![no_main]

use jqesque::{Jqesque, JqesqueError, LimitKind, ParseOptions, Separator, DEFAULT_MAX_PATH_DEPTH};
use libfuzzer_sys::fuzz_target;
use serde_json::json;

fuzz_target!(|data: &[u8]| {
    if data.is_empty() || data.len() > 4096 {
        return;
    }

    let depth = data[0] as usize + 1;
    let shape = data.get(1).copied().unwrap_or_default() % 3;
    let depth_limit = match data.get(2).copied().unwrap_or_default() % 5 {
        0 => 0,
        1 => 2,
        2 => 64,
        3 => DEFAULT_MAX_PATH_DEPTH,
        _ => usize::MAX,
    };
    let index = data.iter().skip(3).fold(0usize, |acc, b| {
        acc.saturating_mul(10).saturating_add((b % 10) as usize)
    });

    for separator in [Separator::Dot, Separator::Slash, Separator::Custom(':')] {
        let path = if shape == 0 {
            "[0]".repeat(depth)
        } else {
            std::iter::repeat_n(if shape == 1 { "a" } else { "\"a.b\"" }, depth)
                .collect::<Vec<_>>()
                .join(&separator.as_char().to_string())
        };
        let input = format!(">{}[{}]=1", path, index);
        let options = ParseOptions::new(separator)
            .strict_json_values(false)
            .max_path_depth(depth_limit)
            .max_array_index(256);

        let result = Jqesque::from_str_with_options(&input, options);
        let effective_limit = depth_limit.min(DEFAULT_MAX_PATH_DEPTH);
        if depth + 1 > effective_limit {
            assert!(matches!(
                result,
                Err(JqesqueError::LimitExceededError {
                    kind: LimitKind::PathDepth,
                    limit,
                    found,
                }) if limit == effective_limit && found == effective_limit + 1
            ));
        } else if index > 256 {
            assert!(matches!(
                result,
                Err(JqesqueError::LimitExceededError {
                    kind: LimitKind::ArrayIndex,
                    ..
                })
            ));
        } else {
            let parsed = result.expect("generated path is within the configured limits");
            assert_eq!(parsed.tokens().len(), depth + 1);
            let _ = parsed.as_json();
            let mut json_obj = json!({
                "seed": [1, 2, 3],
                "settings": { "theme": "light" }
            });
            let _ = parsed.apply_to(&mut json_obj);
        }
    }
});
