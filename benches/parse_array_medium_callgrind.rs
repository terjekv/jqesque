use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use jqesque::{Jqesque, ParseOptions, Separator};

#[library_benchmark]
fn parse_array_medium() -> Option<usize> {
    let options = ParseOptions::new(Separator::Dot)
        .strict_json_values(false)
        .max_path_depth(64)
        .max_array_index(10_000);

    let input = ">items=[1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31,32]";
    let parsed = Jqesque::from_str_with_options(input, options).ok()?;
    Some(parsed.tokens().len())
}

library_benchmark_group!(name = parse_array_medium_group; benchmarks = parse_array_medium);
main!(library_benchmark_groups = parse_array_medium_group);
