// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package sfvm

import (
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca/vm"
	"github.com/stretchr/testify/require"
)

func TestComputeStackUsage_ProducesValidResultsForSingleOps(t *testing.T) {
	tests := []struct {
		op    vm.OpCode
		usage stackUsage
	}{
		{vm.STOP, stackUsage{from: 0, to: 0, delta: 0}},
		{vm.ADD, stackUsage{from: -2, to: 0, delta: -1}},
		{vm.POP, stackUsage{from: -1, to: 0, delta: -1}},
		{vm.PUSH5, stackUsage{from: 0, to: 1, delta: 1}},
		{vm.SWAP1, stackUsage{from: -2, to: 0, delta: 0}},
		{vm.SWAP10, stackUsage{from: -11, to: 0, delta: 0}},
		{vm.DUP1, stackUsage{from: -1, to: 1, delta: 1}},
		{vm.DUP12, stackUsage{from: -12, to: 1, delta: 1}},
		{vm.LOG3, stackUsage{from: -5, to: 0, delta: -5}},
	}

	for _, test := range tests {
		t.Run(test.op.String(), func(t *testing.T) {
			usage := computeStackUsage(test.op)
			if got, want := usage, test.usage; got != want {
				t.Errorf("unexpected result: want %v, got %v", want, got)
			}
		})
	}
}

func TestComputeStackUsage_MatchesTheYellowPaperForEveryOpCode(t *testing.T) {
	require := require.New(t)
	// Stack items removed (δ) and added (α) per opcode, Yellow Paper Appendix H
	// plus the EIPs introducing PUSH0, TLOAD, TSTORE, MCOPY, BLOBHASH,
	// BLOBBASEFEE and CLZ.
	type popsPushes struct{ pops, pushes int }
	spec := map[vm.OpCode]popsPushes{
		vm.STOP: {0, 0}, vm.ADD: {2, 1}, vm.MUL: {2, 1}, vm.SUB: {2, 1}, vm.DIV: {2, 1},
		vm.SDIV: {2, 1}, vm.MOD: {2, 1}, vm.SMOD: {2, 1}, vm.ADDMOD: {3, 1}, vm.MULMOD: {3, 1},
		vm.EXP: {2, 1}, vm.SIGNEXTEND: {2, 1},
		vm.LT: {2, 1}, vm.GT: {2, 1}, vm.SLT: {2, 1}, vm.SGT: {2, 1}, vm.EQ: {2, 1},
		vm.ISZERO: {1, 1}, vm.AND: {2, 1}, vm.OR: {2, 1}, vm.XOR: {2, 1}, vm.NOT: {1, 1},
		vm.BYTE: {2, 1}, vm.SHL: {2, 1}, vm.SHR: {2, 1}, vm.SAR: {2, 1}, vm.CLZ: {1, 1},
		vm.SHA3:    {2, 1},
		vm.ADDRESS: {0, 1}, vm.BALANCE: {1, 1}, vm.ORIGIN: {0, 1}, vm.CALLER: {0, 1},
		vm.CALLVALUE: {0, 1}, vm.CALLDATALOAD: {1, 1}, vm.CALLDATASIZE: {0, 1},
		vm.CALLDATACOPY: {3, 0}, vm.CODESIZE: {0, 1}, vm.CODECOPY: {3, 0}, vm.GASPRICE: {0, 1},
		vm.EXTCODESIZE: {1, 1}, vm.EXTCODECOPY: {4, 0}, vm.RETURNDATASIZE: {0, 1},
		vm.RETURNDATACOPY: {3, 0}, vm.EXTCODEHASH: {1, 1},
		vm.BLOCKHASH: {1, 1}, vm.COINBASE: {0, 1}, vm.TIMESTAMP: {0, 1}, vm.NUMBER: {0, 1},
		vm.PREVRANDAO: {0, 1}, vm.GASLIMIT: {0, 1}, vm.CHAINID: {0, 1}, vm.SELFBALANCE: {0, 1},
		vm.BASEFEE: {0, 1}, vm.BLOBHASH: {1, 1}, vm.BLOBBASEFEE: {0, 1},
		vm.POP: {1, 0}, vm.MLOAD: {1, 1}, vm.MSTORE: {2, 0}, vm.MSTORE8: {2, 0}, vm.SLOAD: {1, 1},
		vm.SSTORE: {2, 0}, vm.JUMP: {1, 0}, vm.JUMPI: {2, 0}, vm.PC: {0, 1}, vm.MSIZE: {0, 1},
		vm.GAS: {0, 1}, vm.JUMPDEST: {0, 0}, vm.TLOAD: {1, 1}, vm.TSTORE: {2, 0}, vm.MCOPY: {3, 0},
		vm.PUSH0:  {0, 1},
		vm.CREATE: {3, 1}, vm.CALL: {7, 1}, vm.CALLCODE: {7, 1}, vm.RETURN: {2, 0},
		vm.DELEGATECALL: {6, 1}, vm.CREATE2: {4, 1}, vm.STATICCALL: {6, 1}, vm.REVERT: {2, 0},
		vm.SELFDESTRUCT: {1, 0},
	}
	for op := vm.PUSH1; op <= vm.PUSH32; op++ {
		spec[op] = popsPushes{0, 1}
	}
	for n := 1; n <= 16; n++ {
		spec[vm.DUP1+vm.OpCode(n-1)] = popsPushes{n, n + 1}
		spec[vm.SWAP1+vm.OpCode(n-1)] = popsPushes{n + 1, n + 1}
	}
	for n := 0; n <= 4; n++ {
		spec[vm.LOG0+vm.OpCode(n)] = popsPushes{n + 2, 0}
	}

	want := map[vm.OpCode]stackUsage{}
	got := map[vm.OpCode]stackUsage{}
	for i := range numOpCodes {
		op := vm.OpCode(i)
		// Undefined opcodes (and INVALID) report no stack usage; they fail as
		// invalid instructions instead.
		usage := stackUsage{}
		if s, defined := spec[op]; defined {
			delta := s.pushes - s.pops
			usage = stackUsage{from: -s.pops, to: max(delta, 0), delta: delta}
		}
		want[op] = usage
		got[op] = computeStackUsage(op)
	}

	require.Equal(want, got)
}
