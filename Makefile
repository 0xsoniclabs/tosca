# --------------------------------------------------------------------------
# Makefile for the Fantom Tosca - long format EVM
#
# v1.0 (2023/03/28) - Initial version
#
# (c) Fantom Foundation, 2023
# --------------------------------------------------------------------------

STATICCHECK_VERSION = 2025.1
ERRCHECK_VERSION = v1.9.0

.PHONY: all tosca tosca-go tosca-rust test test-go test-rust \
        bench bench-go clean clean-go clean-rust license-headers

all: tosca

tosca: tosca-go

tosca-go: tosca-rust
	go build ./...

tosca-rust:
	cd rust; \
	cargo build --lib --release --features performance; \
	# we expect the dynamic library to have .so suffix, so we copy it here if on macOS
	# on other platforms the .dylib does not exists and the command fails, so we ignore errors
	cp rust/target/release/libevmrs.dylib rust/target/release/libevmrs.so || true 

tosca-rust-coverage:
	cd rust; \
	RUSTFLAGS="-C instrument-coverage" cargo build --lib --release --features performance

ct-coverage-go: DATE=$(shell date +"%Y-%m-%d-%T")
ct-coverage-go: PACKAGES=${EXTRA_PACKAGES},./go/ct/driver/
ct-coverage-go: export GOCOVERDIR=./go/build/${DATE}
ct-coverage-go:
	@ mkdir -p ${GOCOVERDIR} ;\
	go run -cover -coverpkg ${PACKAGES} ./go/ct/driver run --max-errors 1 ${TOSCA_GO_COVERAGE_EVM} ;\
	go tool covdata textfmt --i ${GOCOVERDIR} -o ${GOCOVERDIR}/cover.out ;\
	go tool cover -html ${GOCOVERDIR}/cover.out -o coverage.html ;\
	echo "Coverage report generated in coverage.html"

ct-coverage-lfvm: TOSCA_GO_COVERAGE_EVM=lfvm
ct-coverage-lfvm: EXTRA_PACKAGES=github.com/0xsoniclabs/tosca/go/interpreter/lfvm
ct-coverage-lfvm: ct-coverage-go

ct-coverage-geth: TOSCA_GO_COVERAGE_EVM=geth
ct-coverage-geth: EXTRA_PACKAGES=github.com/ethereum/go-ethereum/core/vm/...
ct-coverage-geth: ct-coverage-go

test: test-go test-rust

test-go: tosca-go
	@go test ./... -count 1 --coverprofile=cover.out

test-rust:
	cd rust; \
	cargo test --features performance

bench: bench-go

bench-go:
	@go test -bench=. ./...

clean: clean-go clean-rust

clean-go:
	$(RM) -r ./go/build/*

clean-rust:
	cd rust; \
	cargo clean

license-headers:
	cd ./scripts/license; ./add_license_header.sh

fuzz-lfvm:
	go test -fuzz=FuzzLfvm ./go/ct/

fuzz-lfvm-diff:
	go test -fuzz=FuzzDifferentialLfvmVsGeth ./go/ct/

test-coverage: test-go coverage-report

coverage-report:
	@go install github.com/vladopajic/go-test-coverage/v2@v2.10.1
	@go-test-coverage --config .testcoverage.yml

lint-go:
	@go install github.com/golangci/golangci-lint/v2/cmd/golangci-lint@v2.15.0
	@golangci-lint run ./go/...
