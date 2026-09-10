use gungraun::{library_benchmark, library_benchmark_group, main};
use jqesque::{Jqesque, ParseOptions, Separator};
use serde_json::{json, Value};

// Parse and prepare the document outside measurement; include application and drop costs.
fn setup(large: bool) -> (Jqesque, Value) {
    let value = if large {
        json!({"values": (0..512).collect::<Vec<_>>()})
    } else {
        json!(true)
    };
    let input = format!("settings.theme.colors={value}");
    let options = ParseOptions::new(Separator::Dot)
        .strict_json_values(true)
        .max_path_depth(64)
        .max_array_index(10_000);
    (
        Jqesque::from_str_with_options(&input, options).unwrap(),
        json!({}),
    )
}

#[library_benchmark]
#[bench::scalar(setup(false))]
#[bench::object(setup(true))]
fn apply_auto((assignment, mut document): (Jqesque, Value)) -> Value {
    assignment.apply_to(&mut document).unwrap();
    document
}

library_benchmark_group!(name = auto_group; benchmarks = apply_auto);
main!(library_benchmark_groups = auto_group);
