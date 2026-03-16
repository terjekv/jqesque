use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use jqesque::{Jqesque, ParseOptions, Separator};
use serde_json::{json, Value};

#[library_benchmark]
fn merge_array_medium() -> Value {
    let options = ParseOptions::new(Separator::Dot)
        .strict_json_values(false)
        .max_path_depth(64)
        .max_array_index(10_000);

    let parsed = Jqesque::from_str_with_options("~items=[10,11,12,13,14,15,16,17]", options)
        .expect("parse ok");

    let mut json_obj = json!({"items": [1,2,3,4,5,6,7,8]});
    parsed.apply_to(&mut json_obj).expect("apply ok");
    json_obj
}

library_benchmark_group!(name = merge_array_medium_group; benchmarks = merge_array_medium);
main!(library_benchmark_groups = merge_array_medium_group);
