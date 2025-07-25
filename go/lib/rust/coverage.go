// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package rust

/*
#cgo LDFLAGS: -L${SRCDIR}/../../../rust/target/release -levmrs -Wl,-rpath,${SRCDIR}/../../../rust/target/release
#include <stdint.h>
#include <stdlib.h>
void evmrs_dump_coverage(char* filename);
uint8_t evmrs_is_coverage_enabled();
*/
import "C"
import "unsafe"

// isRustCoverageEnabled returns true if Rust has been compiled with coverage enabled.
// This assumes that every Rust library loaded at runtime for which coverage data should
// be collected has been compiled with coverage enabled.
func isRustCoverageEnabled() bool {
	return C.evmrs_is_coverage_enabled() != 0
}

// DumpRustCoverageData triggers the Rust code to dump coverage data.
// Not calling this function will result in no coverage data being reported
// for runtime loaded Rust libraries.
// If coverage data collection is not enabled, this function is a no-op.
func DumpRustCoverageData(filename string) {
	cStr := C.CString(filename)
	defer C.free(unsafe.Pointer(cStr))
	C.evmrs_dump_coverage(cStr)
}
