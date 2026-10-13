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
	"slices"
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/0xsoniclabs/tosca/go/tosca/vm"
	"github.com/stretchr/testify/require"
)

func TestGas_getStaticGasPrices_MatchesTheSpecificationForEveryOpCodeAndRevision(t *testing.T) {
	// Static gas prices as of Istanbul, taken from the Yellow Paper (Appendix G)
	// and EIP-150, EIP-1884, EIP-2200, EIP-3198, EIP-3855, EIP-4844, EIP-5656 and
	// EIP-7939. Opcodes introduced after Istanbul are listed with the price they
	// have from their introduction on; the table is not revision gated.
	istanbul := map[vm.OpCode]tosca.Gas{
		vm.STOP: 0, vm.ADD: 3, vm.MUL: 5, vm.SUB: 3, vm.DIV: 5, vm.SDIV: 5, vm.MOD: 5, vm.SMOD: 5,
		vm.ADDMOD: 8, vm.MULMOD: 8, vm.EXP: 10, vm.SIGNEXTEND: 5,
		vm.LT: 3, vm.GT: 3, vm.SLT: 3, vm.SGT: 3, vm.EQ: 3, vm.ISZERO: 3, vm.AND: 3, vm.OR: 3,
		vm.XOR: 3, vm.NOT: 3, vm.BYTE: 3, vm.SHL: 3, vm.SHR: 3, vm.SAR: 3, vm.CLZ: 5,
		vm.SHA3:    30,
		vm.ADDRESS: 2, vm.BALANCE: 700, vm.ORIGIN: 2, vm.CALLER: 2, vm.CALLVALUE: 2,
		vm.CALLDATALOAD: 3, vm.CALLDATASIZE: 2, vm.CALLDATACOPY: 3, vm.CODESIZE: 2, vm.CODECOPY: 3,
		vm.GASPRICE: 2, vm.EXTCODESIZE: 700, vm.EXTCODECOPY: 700, vm.RETURNDATASIZE: 2,
		vm.RETURNDATACOPY: 3, vm.EXTCODEHASH: 700,
		vm.BLOCKHASH: 20, vm.COINBASE: 2, vm.TIMESTAMP: 2, vm.NUMBER: 2, vm.PREVRANDAO: 2,
		vm.GASLIMIT: 2, vm.CHAINID: 2, vm.SELFBALANCE: 5, vm.BASEFEE: 2, vm.BLOBHASH: 3, vm.BLOBBASEFEE: 2,
		vm.POP: 2, vm.MLOAD: 3, vm.MSTORE: 3, vm.MSTORE8: 3, vm.SLOAD: 800, vm.SSTORE: 0,
		vm.JUMP: 8, vm.JUMPI: 10, vm.PC: 2, vm.MSIZE: 2, vm.GAS: 2, vm.JUMPDEST: 1,
		vm.TLOAD: 100, vm.TSTORE: 100, vm.MCOPY: 3, vm.PUSH0: 2,
		vm.LOG0: 375, vm.LOG1: 750, vm.LOG2: 1125, vm.LOG3: 1500, vm.LOG4: 1875,
		vm.CREATE: 32000, vm.CALL: 700, vm.CALLCODE: 700, vm.RETURN: 0, vm.DELEGATECALL: 700,
		vm.CREATE2: 32000, vm.STATICCALL: 700, vm.REVERT: 0, vm.INVALID: 0, vm.SELFDESTRUCT: 5000,
	}
	for op := vm.PUSH1; op <= vm.PUSH32; op++ {
		istanbul[op] = 3
	}
	for op := vm.DUP1; op <= vm.DUP16; op++ {
		istanbul[op] = 3
	}
	for op := vm.SWAP1; op <= vm.SWAP16; op++ {
		istanbul[op] = 3
	}

	// EIP-2929 replaced the static price of these by warm/cold dynamic pricing.
	zeroedFromBerlin := []vm.OpCode{
		vm.SLOAD, vm.EXTCODECOPY, vm.EXTCODESIZE, vm.EXTCODEHASH, vm.BALANCE,
		vm.CALL, vm.CALLCODE, vm.STATICCALL, vm.DELEGATECALL,
	}

	for revision := tosca.R07_Istanbul; revision <= newestSupportedRevision; revision++ {
		t.Run(revision.String(), func(t *testing.T) {
			require := require.New(t)
			want := map[vm.OpCode]tosca.Gas{}
			got := map[vm.OpCode]tosca.Gas{}
			for i := range numOpCodes {
				op := vm.OpCode(i)
				price, defined := istanbul[op]
				if !defined {
					price = UNKNOWN_GAS_PRICE
				}
				if revision >= tosca.R09_Berlin && slices.Contains(zeroedFromBerlin, op) {
					price = 0
				}
				want[op] = price
				got[op] = getStaticGasPrices(revision).get(op)
			}
			require.Equal(want, got)
		})
	}
}

// --- SStore ---

func TestGas_getDynamicCostsForSstore_exhaustive(t *testing.T) {
	// This test exhaustively checks the computation of the dynamic gas costs for
	// the SSTORE instruction by enumerating every possible input combination
	// and comparing the result with the specification found on
	// https://www.evm.codes.

	// The specification found on https://www.evm.codes is provided in the form
	// of a python code snippet. The following function is a direct translation
	// of the python code to Go, parameterized to allow for easy testing of
	// different revisions.
	makeSpec := func(create, update, touch tosca.Gas) func(s storageStateExample) tosca.Gas {
		return func(s storageStateExample) tosca.Gas {
			// source: https://www.evm.codes
			gas := tosca.Gas(0)
			if s.value == s.current {
				gas = touch
			} else if s.current == s.original {
				if s.original == 0 {
					gas = create
				} else {
					gas = update
				}
			} else {
				gas = touch
			}
			return gas
		}
	}

	specs := map[tosca.Revision]func(storageStateExample) tosca.Gas{
		// source: https://www.evm.codes/?fork=istanbul
		tosca.R07_Istanbul: makeSpec(20000, 5000, 800),
		// source: https://www.evm.codes/?fork=berlin
		tosca.R09_Berlin: makeSpec(20000, 2900, 100),
	}

	// All other revisions inherit the definition from their predecessor.
	specs[tosca.R10_London] = specs[tosca.R09_Berlin]
	specs[tosca.R11_Paris] = specs[tosca.R10_London]
	specs[tosca.R12_Shanghai] = specs[tosca.R11_Paris]
	specs[tosca.R13_Cancun] = specs[tosca.R12_Shanghai]
	specs[tosca.R14_Prague] = specs[tosca.R13_Cancun]
	specs[tosca.R15_Osaka] = specs[tosca.R14_Prague]

	// Check that gas prices are computed correctly.
	for _, revision := range tosca.GetAllKnownRevisions() {
		spec, found := specs[revision]
		if !found {
			t.Errorf("missing specification for revision %v", revision)
			continue
		}
		for storageStatus, example := range getStorageStateExamples() {
			want := spec(example)
			got := getDynamicCostsForSstore(revision, storageStatus)
			if got != want {
				t.Errorf(
					"unexpected result for (%v,%v), wanted %d, got %d",
					revision,
					storageStatus,
					want,
					got,
				)
			}
		}
	}
}

func TestGas_getRefundForSstore_exhaustive(t *testing.T) {
	// This test exhaustively checks the computation of the refunds granted by
	// the SSTORE instruction by enumerating every possible input combination
	// and comparing the result with the specification found on
	// https://www.evm.codes.

	// The specification found on https://www.evm.codes is provided in the form
	// of a python code snippet. The following function is a direct translation
	// of the python code to Go, parameterized to allow for easy testing of
	// different revisions.
	makeSpec := func(delete, resetToZero, resetToNonZero tosca.Gas) func(s storageStateExample) tosca.Gas {
		return func(s storageStateExample) tosca.Gas {
			// source: https://www.evm.codes
			refund := tosca.Gas(0)
			if s.value != s.current {
				if s.current == s.original {
					if s.original != 0 && s.value == 0 {
						refund += delete
					}
				} else {
					if s.original != 0 {
						if s.current == 0 {
							refund -= delete
						} else if s.value == 0 {
							refund += delete
						}
					}
					if s.value == s.original {
						if s.original == 0 {
							refund += resetToZero
						} else {
							refund += resetToNonZero
						}
					}
				}
			}
			return refund
		}
	}

	specs := map[tosca.Revision]func(storageStateExample) tosca.Gas{
		// source: https://www.evm.codes/?fork=istanbul
		tosca.R07_Istanbul: makeSpec(15000, 19200, 4200),
		// source: https://www.evm.codes/?fork=berlin
		tosca.R09_Berlin: makeSpec(15000, 20000-100, 5000-2100-100),
		// source: https://www.evm.codes/?fork=london
		tosca.R10_London: makeSpec(4800, 20000-100, 5000-2100-100),
	}
	// All other revisions inherit the definition from their predecessor.
	specs[tosca.R11_Paris] = specs[tosca.R10_London]
	specs[tosca.R12_Shanghai] = specs[tosca.R11_Paris]
	specs[tosca.R13_Cancun] = specs[tosca.R12_Shanghai]
	specs[tosca.R14_Prague] = specs[tosca.R13_Cancun]
	specs[tosca.R15_Osaka] = specs[tosca.R14_Prague]

	// Check that gas prices are computed correctly.
	for _, revision := range tosca.GetAllKnownRevisions() {
		spec, found := specs[revision]
		if !found {
			t.Errorf("missing specification for revision %v", revision)
			continue
		}
		for storageStatus, example := range getStorageStateExamples() {
			want := spec(example)
			got := getRefundForSstore(revision, storageStatus)
			if got != want {
				t.Errorf(
					"unexpected result for (%v,%v), wanted %d, got %d",
					revision,
					storageStatus,
					want,
					got,
				)
			}
		}
	}
}

// getStorageStateExamples returns a map enumerating all possible storage state
// and mapping them to a tipple if original, current and new values of a storage
// slot that constitutes the associated storage state.
//
// This function is intended for testing storage related gas costs and refunds
// functions by providing a complete set of test-case inputs.
func getStorageStateExamples() map[tosca.StorageStatus]storageStateExample {
	X, Y, Z := 1, 2, 3
	return map[tosca.StorageStatus]storageStateExample{
		tosca.StorageAssigned:         {X, Y, Z},
		tosca.StorageAdded:            {0, 0, Z},
		tosca.StorageDeleted:          {X, X, 0},
		tosca.StorageModified:         {X, X, Z},
		tosca.StorageDeletedAdded:     {X, 0, Z},
		tosca.StorageModifiedDeleted:  {X, Y, 0},
		tosca.StorageDeletedRestored:  {X, 0, X},
		tosca.StorageAddedDeleted:     {0, X, 0},
		tosca.StorageModifiedRestored: {X, Y, X},
	}
}

type storageStateExample struct {
	original, current, value int
}
