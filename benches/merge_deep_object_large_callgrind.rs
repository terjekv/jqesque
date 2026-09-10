use gungraun::{library_benchmark, library_benchmark_group, main};
use jqesque::{Jqesque, ParseOptions, Separator};
use serde_json::{json, Value};

#[library_benchmark]
fn merge_deep_object_large() -> Value {
    let options = ParseOptions::new(Separator::Dot)
        .strict_json_values(false)
        .max_path_depth(64)
        .max_array_index(10_000);

    let parsed = Jqesque::from_str_with_options(
        "~root.a.b.c={\"k01\":1,\"k02\":2,\"k03\":3,\"k04\":4,\"k05\":5,\"k06\":6,\"k07\":7,\"k08\":8}",
        options,
    )
    .expect("parse ok");

    let mut json_obj = json!({
        "root": {
            "a": {
                "b": {
                    "c": {
                        "keep": true,
                        "k00": 0
                    }
                }
            }
        }
    });

    parsed.apply_to(&mut json_obj).expect("apply ok");
    json_obj
}

library_benchmark_group!(
    name = merge_deep_object_large_group;
    benchmarks = merge_deep_object_large
);
main!(library_benchmark_groups = merge_deep_object_large_group);
