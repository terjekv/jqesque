use gungraun::{library_benchmark, library_benchmark_group, main};
use jqesque::{Jqesque, ParseOptions, Separator};
use serde_json::{json, Value};

#[library_benchmark]
fn insert_deep_path_medium() -> Value {
    let options = ParseOptions::new(Separator::Dot)
        .strict_json_values(false)
        .max_path_depth(64)
        .max_array_index(10_000);

    let parsed =
        Jqesque::from_str_with_options(">a.b.c.d.e.f.g.h.i.j.k=42", options).expect("parse ok");

    let mut json_obj = json!({"a": {"b": {"c": {}}}});
    parsed.apply_to(&mut json_obj).expect("apply ok");
    json_obj
}

library_benchmark_group!(
    name = insert_deep_path_medium_group;
    benchmarks = insert_deep_path_medium
);
main!(library_benchmark_groups = insert_deep_path_medium_group);
