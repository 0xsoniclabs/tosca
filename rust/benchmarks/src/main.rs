use benchmarks::RunArgs;
use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    runs: u64,
    benchmark: Benchmark,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum Benchmark {
    StaticOverhead,
    Inc1,
    Inc10,
    Fib1,
    Fib5,
    Fib10,
    Fib15,
    Fib20,
    Sha1,
    Sha10,
    Sha100,
    Sha1000,
    Arithmetic1,
    Arithmetic10,
    Arithmetic100,
    Arithmetic280,
    Memory1,
    Memory10,
    Memory100,
    Memory1000,
    Memory10000,
    AnalysisJumpdest,
    AnalysisStop,
    AnalysisPush1,
    AnalysisPush32,
    All,
    AllShort,
}

fn main() {
    let args = Args::parse();

    let benches: Vec<fn() -> (RunArgs, u32)> = match args.benchmark {
        Benchmark::StaticOverhead => vec![|| RunArgs::static_overhead(1)],
        Benchmark::Inc1 => vec![|| RunArgs::inc(1)],
        Benchmark::Inc10 => vec![|| RunArgs::inc(10)],
        Benchmark::Fib1 => vec![|| RunArgs::fib(1)],
        Benchmark::Fib5 => vec![|| RunArgs::fib(5)],
        Benchmark::Fib10 => vec![|| RunArgs::fib(10)],
        Benchmark::Fib15 => vec![|| RunArgs::fib(15)],
        Benchmark::Fib20 => vec![|| RunArgs::fib(20)],
        Benchmark::Sha1 => vec![|| RunArgs::sha3(1)],
        Benchmark::Sha10 => vec![|| RunArgs::sha3(10)],
        Benchmark::Sha100 => vec![|| RunArgs::sha3(100)],
        Benchmark::Sha1000 => vec![|| RunArgs::sha3(1000)],
        Benchmark::Arithmetic1 => vec![|| RunArgs::arithmetic(1)],
        Benchmark::Arithmetic10 => vec![|| RunArgs::arithmetic(10)],
        Benchmark::Arithmetic100 => vec![|| RunArgs::arithmetic(100)],
        Benchmark::Arithmetic280 => vec![|| RunArgs::arithmetic(280)],
        Benchmark::Memory1 => vec![|| RunArgs::memory(1)],
        Benchmark::Memory10 => vec![|| RunArgs::memory(10)],
        Benchmark::Memory100 => vec![|| RunArgs::memory(100)],
        Benchmark::Memory1000 => vec![|| RunArgs::memory(1000)],
        Benchmark::Memory10000 => vec![|| RunArgs::memory(10000)],
        Benchmark::AnalysisJumpdest => vec![|| RunArgs::jumpdest_analysis(0x6000)],
        Benchmark::AnalysisStop => vec![|| RunArgs::stop_analysis(0x6000)],
        Benchmark::AnalysisPush1 => vec![|| RunArgs::push1_analysis(0x6000)],
        Benchmark::AnalysisPush32 => vec![|| RunArgs::push32_analysis(0x6000)],
        Benchmark::All => vec![
            || RunArgs::static_overhead(1),
            || RunArgs::inc(1),
            || RunArgs::fib(20),
            || RunArgs::sha3(1000),
            || RunArgs::arithmetic(280),
            || RunArgs::memory(10000),
            || RunArgs::jumpdest_analysis(0x6000),
            || RunArgs::stop_analysis(0x6000),
            || RunArgs::push1_analysis(0x6000),
            || RunArgs::push32_analysis(0x6000),
        ],
        Benchmark::AllShort => vec![
            || RunArgs::static_overhead(1),
            || RunArgs::inc(1),
            || RunArgs::fib(1),
            || RunArgs::sha3(1),
            || RunArgs::arithmetic(1),
            || RunArgs::memory(1),
            || RunArgs::jumpdest_analysis(100),
            || RunArgs::stop_analysis(100),
            || RunArgs::push1_analysis(100),
            || RunArgs::push32_analysis(100),
        ],
    };

    for bench_fn in benches {
        let (mut run_args, expected) = bench_fn();
        for _ in 0..args.runs {
            assert_eq!(benchmarks::run(&mut run_args), expected);
        }
    }
}
