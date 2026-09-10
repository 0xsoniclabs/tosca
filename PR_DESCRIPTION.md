Pass hot arguments directly to hold them in registers

Threads the program counter, the gas counter and the stack through the
handler chain as by-value arguments instead of reading them out of the
interpreter struct, so that they live in registers for the whole run and
only touch memory when the chain is parked. With feature `tail-call` a
handler takes `(&mut Frame, Pc, Gas, Stack)` and returns
`Result<(Pc, Gas, Stack), FailStatus>`; without it the calling
convention has no way to return all three in registers, so the dispatch
loop keeps the stack in a slot it lends out and only `(Pc, Gas)` come
back by value.

## Why

With function-pointer dispatch LLVM can never prove the interpreter
struct non-aliased or dead, so SROA cannot promote its fields and every
opcode reloaded `pc`, `gas_left` and the stack header from memory and
stored them back. Changing the calling convention does not help: a
reference is a pointer, and any aggregate over 16 bytes is classified
MEMORY by every x86-64 convention. The only mechanism that puts these
values in registers is making them *arguments*.

The old state is therefore split three ways. `Frame` keeps what stays in
memory while the handlers run and is passed by reference. `Parked` holds
the register state while the handlers do *not* run, i.e. before and
after `Interpreter::run`. `Interpreter` owns the frame plus the stack
buffer.

Each hot value had to be shrunk to one word to be worth passing:

- `CodeReader` is split into `Code` (the byte slice plus its analysis,
  fixed for the whole run, borrowed by the frame) and `Pc` (a single
  pointer or offset, copied by value). `Code` is now built by the caller
  in `evmrs.rs`, which also removes the code-analysis-cache argument
  from both `Interpreter` constructors.
- `Stack` becomes a two-word borrowed view over a `StackBuffer` that the
  interpreter owns and the allocation cache reuses. It occupies the top
  of the buffer and grows *downward*, so element 0 is the top of stack,
  accesses sit at constant offsets, and the length checks double as the
  bounds proofs. The `Vec` bookkeeping is gone, and with it the only
  user of the `unsafe-stack` cfg.

`exit_fn!` is added next to `op_fn!` for the four operations that end a
run (`stop`, `return_`, `revert`, `self_destruct`), because a handler
that ends the run has to hand its final state back rather than dispatch
the next opcode.

With `tail-call` the chain uses `extern "rust-preserve-none"`, which has
argument registers to spare for the returned state and leaves nothing
callee-saved for the handlers to preserve. Handing the state on is then
free: it is already in the argument registers of the tail call.

**Returning it is not.** Without `tail-call` a handler returns to the
dispatch loop, and the return value is where this design has to stop:
the ABI hands back a *pair of scalars* in `%rax`/`%rdx`, and
`Result<(Pc, Gas, Stack), FailStatus>` is 32 bytes, so it travels
through a slot in the caller's frame. That costs the handler four
stores, costs the loop four loads to put the state back into argument
registers, and pins all six argument registers, `%rdi` among them as
the pointer to the return slot.

So the loop-based dispatcher keeps only two of the three in registers.
A handler takes `(&mut Frame, Pc, Gas, &mut Stack)` and returns
`Option<(Pc, Gas)>`: the stack lives in a slot the loop owns and lends
out by reference, and the pair of scalars comes back in `%rax`/`%rdx` -
with `fn-ptr-conversion-dispatch`, where `Pc` is a pointer and has a
niche; in the jumptable configuration it is a `usize` and the `Option`
is 24 bytes again.
It has to be `Option`, not `Result` - 16 bytes is not sufficient, the
layout has to *be* a scalar pair, and a `Result` needs to put the
failure somewhere in those two words. `Option` does not: the niche of
the `Pc` pointer carries `None` without a field of its own. `None`
therefore means only "the run is over", the frame says why, and
`fail` records a `FailStatus` for the loop on the way out.

That also removes the loop's second exit test. `main` checks the status
byte a handler returns *and* `frame.exec_status`, because an exit
handler reports through the frame while a failure reports through the
return value. Here both end the chain the same way, so one test on the
returned `Pc` covers both, and the four exit handlers park the state
they ended with for the loop to pick up.

## Codegen

`--features performance` on rustc 1.98.1 and `--features
performance-nightly` on 1.99.0-nightly (2026-08-05) for `tail-call`.
The dispatch loop without `tail-call`, before and after:

    ; before                            ; after
    mov    0x338(%rsp),%rax             mov    %r14,%rdi
    mov    %r14,%rdi                    mov    %rax,%rsi
    vzeroupper                          mov    %r15,%rcx
    call   *0x28(%rax)                  vzeroupper
    cmp    $0xfc,%al                    call   *0x28(%rax)
    jne    <failed>                     test   %rax,%rax
    movzbl 0x36c(%rsp),%eax             jne    <loop>
    test   %al,%al
    jne    <run over>

Nine instructions per dispatched opcode become seven, and what they do
changes: `main` reloads the pc from the interpreter struct and tests two
exit conditions, while the new loop passes on the pc it just got back
in `%rax`, the gas counter in `%rdx` where the next handler already
expects it, and a pointer to the stack slot in `%rcx`. (In the shipped
cdylib `main`'s loop is eight instructions, because LLVM folds the
`exec_status` load into a `cmpb` there; the listing is from the
benchmark binary the numbers below are measured on.)

The handlers pay some of that back and save more elsewhere. Instructions
per execution on fib20, counted exactly with callgrind: `push1`
21 -> 17, `swap1` 18 -> 14, `dup3` 21 -> 19, `jump` 37 -> 31,
`is_zero` 22 -> 18, `and` 21 -> 20, `pop` 13 -> 14 and `jump_i`
46.4 -> 50.0, the one grower. `pop` gains an instruction but its memory
accesses go 6 -> 4: `main` reads and writes the gas counter and the
stack length in the interpreter and updates the pc in place, where the
new code only reads and writes the two stack words in the loop's slot.
Over the 124 `gen_jumptable` symbols of that build, instructions from
entry to the first unconditional transfer go 4749 -> 4686, symbols
carrying a stack frame 57 -> 61, `.text` +3104 B.

With `tail-call` nothing returns at all and the whole state stays in
registers. Over the 238 symbols of that build the hot total goes
9622 -> 7956; `pop` 14 -> 10, `push1` 20 -> 13, `swap1` 19 -> 13,
`jump` 36 -> 28. Frames 111 -> 109: nothing is callee-saved under
preserve-none, so a handler that makes a non-tail host call spills the
threaded state itself - `ext_code_copy` is the one real grower at
60 -> 98, its `(%rsp)` references going 18 -> 42 - while every leaf
handler loses its prologue. `.text` +2768 B.

The `StackBuffer` costs nothing per execution: with `alloc-reuse`, which
both `performance` builds enable, it comes from the pool, and `execute`
contains no allocation call and no `memset` at all. Under default
features the only `memset` left in `execute` fills the code analysis
with `CodeByteType::DataOrInvalid` over the length of the code.

## Measured effect

benchmarks binary under perf stat, pinned to one Zen5 core; against
main @ dc11abd, both binaries of a column built by one compiler. 20
rounds with the two states interleaved and their order rotated per
round, `setarch --addr-no-randomize`, a `runs=0` startup measurement
subtracted per round, machine otherwise idle. Instructions are medians
per run, reproducible to ±0.01%; cycles are paired per-round ratios
with t-based 95% CIs. * marks an interval that includes zero or the
benchmark's own code-layout spread (~0.5%, 1% on static-overhead, 2-5%
on sha1000), i.e. no resolvable change.

                  performance             performance-nightly
    benchmark         insns  cycles             insns  cycles
    static-overhead   -4.6%   -6.4% ±0.42      -7.0%   -3.3% ±1.02
    inc10            -12.8%  -17.3% ±0.96     -23.6%  -12.7% ±0.67
    fib20            -15.4%  -14.3% ±0.71     -27.8%  -23.0% ±0.71
    arithmetic280     -7.8%  -13.5% ±1.38     -19.2%   -0.8% ±1.00 *
    memory10000      -10.1%  -10.2% ±1.03     -23.4%  -15.7% ±0.78
    sha1000           -7.9%   -7.2% ±1.03     -16.2%   -7.9% ±1.46
    analysis-push32   -5.7%   -9.3% ±0.71      -8.2%   -2.3% ±1.76 *

The nightly build is unaffected by the loop-dispatcher change, which is
the point of confining it to the wrapper: measured against the previous
revision of this branch over 10 rounds, instruction counts differ by at
most 0.02% on the interpretation-bound rows (-0.5% on static-overhead,
where the parked state moved into the frame) and every cycle interval
covers zero.

Both configurations improve on every benchmark of the set.
`performance-nightly` removes 7-28% of the instruction stream, and
between 3% and 23% of the cycles on five of seven rows - fib20, the
longest interpretation-bound workload of the set, by 23.0%. Under plain
`performance` the loop pays for what it cannot keep in registers, so
the instruction win is roughly half of it, 4.6-15.4%, and cycles follow
at 6.4-17.3%.

The channel is the instruction count and not memory latency. Failed
store-to-load forwarding (`ls_bad_status2.stli_other`, raw event
`r0224`) *rises* against main in the loop configuration - fib20 +27.5%
±1.6, arithmetic280 +52.1% ±1.0, sha1000 +59.6% ±5.2, inc10 +5.0% ±1.9
over six interleaved rounds - because the stack now bounces through the
loop's slot every opcode, and it costs nothing at this IPC. Any account
of this branch in terms of that counter, including the one in an
earlier revision of this description, is wrong.

The two rows that regressed in that revision are the ones this fixes.
arithmetic280, whose handlers hold the widest live sets, was +2.6% and
+4.0% cycles in two sweeps and is now -13.5%; sha1000, the outlier that
revision could not explain, was +18% to +20% and is now -7.2%.

## Behavior

`Stack::as_slice` flips to top-of-stack-first. `Stack::pop`,
`pop_with_location`, `dup` and `swap_with_top` keep their existing
argument order and semantics, and `StepResult.stack` is still reported
bottom-first, now via `Stack::iter_from_bottom`, so EVM-observable
behavior is unchanged.

The one `unsafe` the loop dispatcher adds is a `ptr::read` per opcode,
moving the stack out of the slot the loop lends out; putting it back is
an ordinary assignment. `Stack` has no `Drop`, so the copy left behind
on a failure path is never used or released. Miri passes with the CI flags, and the
lib tests pass under strict Stacked Borrows as well; the two strict
failures - `types::amount::tests::display`, inside `ethnum`'s `Display`
impl, and the `ffi` integration test - reproduce unchanged on main.

Passing:

    cargo test
    cargo test --features fn-ptr-conversion-dispatch
    cargo test --features performance
    cargo +nightly-2026-08-05 test --profile release --features tail-call
    cargo +nightly-2026-08-05 test --profile release --features tail-call,fn-ptr-conversion-dispatch
    cargo +nightly-2026-08-05 test --profile release --features simd
    cargo +nightly-2026-08-05 test --profile release --features performance-nightly
    cargo clippy --all-targets [--features <each>] -- --deny warnings
    cargo +nightly-2026-08-05 miri test    (CI flags, and strict Stacked Borrows)
    go test ./go/integration_test/interpreter/
    go run ./go/ct/driver regressions evmrs
    go run ./go/ct/driver run -f "(stop|return|revert|self_destruct|jump|invalid|gas)" evmrs

Not verified here: the full conformance-test run, and the Go benchmark
suite through the cgo bridge - the numbers above are the Rust
benchmarks binary, which does not pay for the EVMC crossing.
