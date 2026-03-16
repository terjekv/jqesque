use iai_callgrind::{library_benchmark, library_benchmark_group, main};
use jqesque::{Jqesque, ParseOptions, Separator};

#[library_benchmark]
fn parse_deep_path_medium() -> Option<usize> {
    let options = ParseOptions::new(Separator::Dot)
        .strict_json_values(false)
        .max_path_depth(64)
        .max_array_index(10_000);

    let input = ">a.b.c.d.e.f.g.h.i.j.k.l.m.n.o.p=1";
    let parsed = Jqesque::from_str_with_options(input, options).ok()?;
    Some(parsed.tokens().len())
}

library_benchmark_group!(
    name = parse_deep_path_medium_group;
    benchmarks = parse_deep_path_medium
);
main!(library_benchmark_groups = parse_deep_path_medium_group);
