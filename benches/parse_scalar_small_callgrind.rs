use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use jqesque::{Jqesque, ParseOptions, Separator};

#[library_benchmark]
fn parse_scalar_small() -> Option<usize> {
    let options = ParseOptions::new(Separator::Dot)
        .strict_json_values(false)
        .max_path_depth(64)
        .max_array_index(10_000);

    let parsed = Jqesque::from_str_with_options(">a=1", options).ok()?;
    Some(parsed.tokens().len())
}

library_benchmark_group!(name = parse_scalar_small_group; benchmarks = parse_scalar_small);
main!(library_benchmark_groups = parse_scalar_small_group);
