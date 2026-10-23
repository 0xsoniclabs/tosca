# Tosca Build / Development Instructions

Be sure to initialize all submodules

```sh
git submodule update --init --recursive
```

## Build Requirements

- Go toolchain, minimum version 1.27
    - Ubuntu/Debian package: `golang-go`
    - Snap package: `go`
- C toolchain and libclang, needed by cgo and the Rust EVMC bindings
    - Ubuntu/Debian package: `clang`
- Rust toolchain >= 1.97.0
    - [install Rust](https://rustup.rs/)
    - Recommended: install `rust-analyzer` VSCode extension
- [mockgen](https://github.com/golang/mock)
    - Install via Go:
      ```sh
      go install github.com/golang/mock/mockgen@v1.6.0
      ```

### Go Setup Remarks

Note that in some package managers (e.g. apt) the default Go package is an outdated version which is not sufficient for Tosca. Newer versions can be installed by explicitly specifying the Go version, such as `golang-1.27`. Check your currently installed version with the command `go version`. Depending on your installation process it might be required to set `GOROOT`, this can be done in your `.bashrc`.

If no packages are available, follow the instructions on the official [go website](https://go.dev/doc/install) to download and install the newest version.

Go installs programs into `$GOPATH/bin`, where `GOPATH` defaults to `$HOME/go`, add this `bin` directory to your `PATH`. 

## Building

Use the provided Makefile to build and test the project.

```sh
make
make test
```

## Running Benchmarks

The Tosca project includes a set of benchmarks covering a range of EVM implementations and variants thereof. The benchmarks are part of the `github.com/Fantom-foundation/Tosca/go/vm/test` module.

The benchmarks are implemented using Go's benchmark infrastructure. For a comprehensive introduction see the corresponding chapter in [Practical Go Lessons](https://www.practical-go-lessons.com/chap-34-benchmarks) or the [Go Test Package Documentation](https://pkg.go.dev/testing#hdr-Benchmarks).

To run all benchmarks, use the following command:

```sh
go test ./go/vm/test -run=NONE -bench=.
```

The path `./go/vm/test` points to the Go package containing the benchmarks. The flag `-run=NONE` disables the execution of unit tests, and `-bench=<regex>` enables the execution of benchmarks.

Optionally, to include memory usage metrics, the `-benchmem` flag can be added:

```sh
go test ./go/vm/test -run=NONE -bench=. -benchmem
```

The command produces benchmark results in a table format:

```
pkg: github.com/Fantom-foundation/Tosca/go/vm/test
cpu: Intel(R) Core(TM) i7-5820K CPU @ 3.30GHz
BenchmarkInc/1/geth-12  	  121488	     10529 ns/op	    1768 B/op	      16 allocs/op
```

The columns are as follows:
 - the name of the benchmark, using `/` as a separator between benchmarks and sub-benchmarks. In Tosca's setup, the first part of the name is the name of the benchmark (e.g. `BenchmarkInc`), the second part the input size (e.g. `10`), and the last part the EVM implementation (e.g. `sfvm`) followed by the number of cores used to run the benchmark (e.g. `-12`); the latter is added by Go's benchmark infrastructure and has no effect if the benchmark is not using multiple go routines.
 - the number of times the benchmark was executed
 - the average time per execution
 - the average number of bytes allocated per execution (only when using `-benchmem`)
 - the average number of allocations per execution (only when using `-benchmem`)

### Filtering Benchmarks

The `-bench` flag can be used to filter benchmarks by their name using regex expressions. It's important to note that:
 - tests are selected if the regex matches any part of the benchmark name
 - before the regex expressions are applied, the pattern is divided into sub-patterns for sub-tests using the `/` separator

Thus, the command

```sh
go test ./go/vm/test -run=NONE -bench=Inc/1/sf
```

is running the benchmarks

```
BenchmarkInc/1/sfvm-12
BenchmarkInc/10/sfvm-12
```

since the individual parts of the benchmark name are matched one-by-one.

### Profiling Benchmarks

Go has an integrated CPU profiler that can be enabled using the `-cpuprofile` flag:

```sh
go test ./go/vm/test -run=NONE -bench Fib/20/sfvm -cpuprofile cpu.log
```

This command runs the benchmark and collects CPU performance data which can be shown in tabular form using

```sh
go tool pprof -text -nodecount=10 cpu.log
```

or can be visualized by the following command (requires the graphviz library)

```sh
go tool pprof -http "localhost:8000" ./cpu.log
```

which starts a web-server at port `8000` hosting an interactive set of charts to investigate the collected profiling data.

If symbols for native libraries such as evmrs are missing, make sure the library is built including those symbols and that in some go file in the `./go/vm/test` package the following import is present:

```
_ "github.com/ianlancetaylor/cgosymbolizer"
```

This package makes C/C++ symbols accessible to the Go profiler. On some systems, however, this may hide Go symbols so the import may have to be added/removed as needed.

### Diffing Benchmarks

To compare the benchmark results of two different code versions, the [benchstat](https://pkg.go.dev/golang.org/x/perf/cmd/benchstat) tool can be utilized. To install the tool run

```sh
go install golang.org/x/perf/cmd/benchstat@latest
```

which places the binary `benchstat` into your `$GOPATH/bin` directory.

The tool produces reports on the impact of code changes on the performance of benchmarks in the following format:

```
name            old time/op  new time/op  delta
Fib/20/sfvm-12  21.7ms ± 3%  21.1ms ± 2%  -2.66%  (p=0.004 n=9+8)
```

For details on how to use it, please refer to the [Example](https://pkg.go.dev/golang.org/x/perf/cmd/benchstat#hdr-Example) section of the tool's documentation page.

Ideally, pull requests targeting performance improvements should include such a report in their description to document its impact.

## Code Coverage

The Tosca project allows to collect coverage reports for unit tests and CT runs. 
More about coverage in [the docs folder](docs/coverage.md)

### CT Driver Code coverage

To run the the CT driver code coverage simply run `make ct-coverage-sfvm` or `make ct-coverage-geth`, these commands will create the folder `Tosca/go/build/coverage/` and a sub folder with the corresponding interpreter's name. In this directory an instrumented version of the driver will be built and then run. The reports of the coverage of this run will also be in this directory, where lastly an HTML version of the report will be added as well. As more interpreter implementations are added to the Tosca project, they should also be added to the makefile.



## Fuzzing

Go provides fuzzing support through its standard library. Fuzzing attempts to find bugs by mutating a test input data set and measuring test coverage. This complementary technique aims to identify stability issues.			 

### Fuzzing Evm.StepN interface

The ct Evm.StepN interface is used to evaluate N instructions in different EVM implementations. There are 2 Fuzzer tests against this interface:

- Crash test: FuzzSfvm will execute instructions one at the time looking for a panic. ```make fuzz-sfvm```
- Differential tests; execute instruction in two VM implementations and compare state results in addition to panic.
   - FuzzDifferentialSfvmVsGeth: ```make fuzz-sfvm-diff```


## Static analysis

Tosca project uses static analysis to improve code quality.

### Go

Currently, two different static analysis tools are used:
- [Vet](https://pkg.go.dev/cmd/vet) Is the default go linter, bundled with the toolchain. Vet uses heuristics that do not guarantee all reports are genuine problems, but it can find errors not caught by the compilers. To execute it run `go vet ./go/...`. This tool is currently enabled in the CI, and pull request will be prevented from merging when checks are not satisfied.
- [StaticCheck](https://staticcheck.dev/) Finds other [errors](https://staticcheck.dev/docs/checks/) related to styling and good practices. 

Lints can be disabled as described [here](https://golangci-lint.run/usage/false-positives/). Please use `//nolint:pass` where pass is the linter pass that discovers the error. Do not ignore all passes as one line may hide more than one error.
