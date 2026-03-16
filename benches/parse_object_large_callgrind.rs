use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use jqesque::{Jqesque, ParseOptions, Separator};

#[library_benchmark]
fn parse_object_large() -> Option<usize> {
    let options = ParseOptions::new(Separator::Dot)
        .strict_json_values(false)
        .max_path_depth(64)
        .max_array_index(10_000);

    let input = ">settings={\"k01\":1,\"k02\":2,\"k03\":3,\"k04\":4,\"k05\":5,\"k06\":6,\"k07\":7,\"k08\":8,\"k09\":9,\"k10\":10,\"k11\":11,\"k12\":12,\"k13\":13,\"k14\":14,\"k15\":15,\"k16\":16}";
    let parsed = Jqesque::from_str_with_options(input, options).ok()?;
    Some(parsed.tokens().len())
}

library_benchmark_group!(name = parse_object_large_group; benchmarks = parse_object_large);
main!(library_benchmark_groups = parse_object_large_group);
