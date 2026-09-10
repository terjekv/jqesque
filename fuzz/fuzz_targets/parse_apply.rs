#![no_main]

use jqesque::{ApplyOptions, Batch, Jqesque, JqesqueError, LimitKind, ParseOptions, Separator};
use libfuzzer_sys::fuzz_target;
use serde_json::json;

fuzz_target!(|data: &[u8]| {
    if data.len() > 4096 {
        return;
    }
    // Stored assignments are another untrusted boundary, including explicit null values.
    if let Ok(stored) = serde_json::from_slice::<Jqesque>(data) {
        let mut target = json!({"seed": [1, 2, 3]});
        let _ = stored.apply_to_with_options(&mut target, ApplyOptions::new().max_array_slots(256));
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

                let _ = parsed.to_document();
                let _ = parsed.to_json_patch();
                let encoded = serde_json::to_value(&parsed).unwrap();
                let decoded: Jqesque = serde_json::from_value(encoded).unwrap();
                assert_eq!(decoded, parsed);
                let before = json_obj.clone();
                if parsed.apply_to(&mut json_obj).is_err() {
                    assert_eq!(json_obj, before);
                }
                let batch = Batch::new(vec![parsed]).unwrap();
                let mut atomic = before.clone();
                if batch
                    .apply_atomically(&mut atomic, ApplyOptions::new().max_array_slots(64))
                    .is_err()
                {
                    assert_eq!(atomic, before);
                }
            }
            Err(JqesqueError::LimitExceededError {
                kind: LimitKind::PathDepth,
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
