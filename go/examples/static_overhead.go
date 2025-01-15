// Copyright (c) 2024 Fantom Foundation
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at fantom.foundation/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package examples

import (
	"github.com/ethereum/go-ethereum/core/vm"
)

// Create arguments for the interpreter that outline the worst case when calling the
// interpreter with short running contracts. In particular those arguments try to trigger
// all possible allocations that happen when using an evmc interpreter which needs to copy data
// into new allocations.
// - the execution message contains non-empty input
// - the code is not empty so a new allocation is necessary for the code analysis result
// - MStore makes sure memory is not empty
// - Return makes sure output is non-empty
func GetStaticOverheadExample() Example {
	code := []byte{
		byte(vm.PUSH1), 4, // push offset 4
		byte(vm.CALLDATALOAD), // load value from call data at offset 4
		byte(vm.PUSH1), 0,     // push offset 0
		byte(vm.MSTORE),    // store loaded call data value at offset 0
		byte(vm.PUSH1), 32, // push len 32
		byte(vm.PUSH1), 0, // push offset 0
		byte(vm.RETURN), // return 32 bytes at offset 0
	}

	return exampleSpec{
		Name:      "static_overhead",
		Code:      code,
		reference: StaticOverheadRef,
	}.build()
}

func StaticOverheadRef(x int) int {
	return int(x)
}
