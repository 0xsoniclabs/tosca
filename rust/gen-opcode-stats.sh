#!/bin/bash

mkdir -p opcode-stats

BENCHES=(static-overhead inc1 inc10 fib1 fib5 fib10 fib15 fib20 sha1 sha10 sha100 sha1000 arithmetic1 arithmetic10 arithmetic100 arithmetic280 memory1 memory10 memory100 memory1000 memory10000 analysis-jumpdest analysis-stop analysis-push1 analysis-push32 all all-short)

for BENCH in "${BENCHES[@]}"; do
    echo $BENCH
    cargo run --package benchmarks --profile profiling --features performance -- 1 $BENCH > opcode-stats/$BENCH
done
