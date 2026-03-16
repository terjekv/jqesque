use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use jqesque::{Jqesque, ParseOptions, Separator};
use serde_json::{json, Value};

#[library_benchmark]
fn insert_array_dense_large() -> Value {
    let options = ParseOptions::new(Separator::Dot)
        .strict_json_values(false)
        .max_path_depth(64)
        .max_array_index(10_000);

    let parsed = Jqesque::from_str_with_options(
        ">items=[{\"id\":1},{\"id\":2},{\"id\":3},{\"id\":4},{\"id\":5},{\"id\":6},{\"id\":7},{\"id\":8}]",
        options,
    )
    .expect("parse ok");

    let mut json_obj = json!({"items": []});
    parsed.apply_to(&mut json_obj).expect("apply ok");
    json_obj
}

library_benchmark_group!(
    name = insert_array_dense_large_group;
    benchmarks = insert_array_dense_large
);
main!(library_benchmark_groups = insert_array_dense_large_group);
