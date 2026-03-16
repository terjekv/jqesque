use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use jqesque::{Jqesque, ParseOptions, Separator};
use serde_json::{json, Value};

#[library_benchmark]
fn insert_object_small() -> Value {
    let options = ParseOptions::new(Separator::Dot)
        .strict_json_values(false)
        .max_path_depth(64)
        .max_array_index(10_000);

    let parsed =
        Jqesque::from_str_with_options(">settings.theme.color=blue", options).expect("parse ok");

    let mut json_obj = json!({"settings": {"theme": {"font": "mono"}}});
    parsed.apply_to(&mut json_obj).expect("apply ok");
    json_obj
}

library_benchmark_group!(name = insert_object_small_group; benchmarks = insert_object_small);
main!(library_benchmark_groups = insert_object_small_group);
