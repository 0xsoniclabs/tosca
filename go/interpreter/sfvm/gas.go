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
	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/0xsoniclabs/tosca/go/tosca/vm"
)

const (
	CallNewAccountGas    tosca.Gas = 25000 // Paid for CALL when the destination address didn't exist prior.
	CallValueTransferGas tosca.Gas = 9000  // Paid for CALL when the value transfer is non-zero.
	CallStipend          tosca.Gas = 2300  // Free gas given at beginning of call.

	ColdSloadCostEIP2929         tosca.Gas = 2100 // Cost of cold SLOAD after EIP 2929
	ColdAccountAccessCostEIP2929 tosca.Gas = 2600 // Cost of cold account access after EIP 2929

	SloadGasEIP2200                   tosca.Gas = 800   // Cost of SLOAD after EIP 2200 (part of Istanbul)
	SstoreClearsScheduleRefundEIP2200 tosca.Gas = 15000 // Once per SSTORE operation for clearing an originally existing storage slot

	SstoreResetGasEIP2200      tosca.Gas = 5000  // Once per SSTORE operation from clean non-zero to something else
	SstoreSetGasEIP2200        tosca.Gas = 20000 // Once per SSTORE operation from clean zero to non-zero
	WarmStorageReadCostEIP2929 tosca.Gas = 100   // Cost of reading warm storage after EIP 2929

	UNKNOWN_GAS_PRICE = 999999
)

var static_gas_prices = newOpCodePropertyMap(getStaticGasPriceInternal)
var static_gas_prices_berlin = newOpCodePropertyMap(getBerlinGasPriceInternal)

// numOpCodes is the number of opcodes in the EVM.
const numOpCodes = 256

// opCodePropertyMap is a generic property map for precomputed values.
// Its purpose is to provide a precomputed lookup table for OpCode properties
// that can be generated from a function that takes an OpCode as input.
// Using this type hides internal details of the opcode implementation.
type opCodePropertyMap[T any] struct {
	lookup [numOpCodes]T
}

// newOpCodePropertyMap creates a new OpCode property map.
// The property function shall be resilient to undefined OpCode values, and not
// panic. The zero values or a sentinel value shall be used in such cases.
func newOpCodePropertyMap[T any](property func(op vm.OpCode) T) opCodePropertyMap[T] {
	lookup := [numOpCodes]T{}
	for i := 0; i < numOpCodes; i++ {
		lookup[i] = property(vm.OpCode(i))
	}
	return opCodePropertyMap[T]{lookup}
}

func (p *opCodePropertyMap[T]) get(op vm.OpCode) T {
	// Index may be out of bounds. Nevertheless, bounds check carry a performance
	// penalty. If the property map is initialized correctly, the index will be
	// within bounds.
	return p.lookup[op]
}

func getBerlinGasPriceInternal(op vm.OpCode) tosca.Gas {
	gp := getStaticGasPriceInternal(op)

	// Changed static gas prices with EIP2929
	switch op {
	case vm.SLOAD:
		gp = 0
	case vm.EXTCODECOPY:
		gp = 0
	case vm.EXTCODESIZE:
		gp = 0
	case vm.EXTCODEHASH:
		gp = 0
	case vm.BALANCE:
		gp = 0
	case vm.CALL:
		gp = 0
	case vm.CALLCODE:
		gp = 0
	case vm.STATICCALL:
		gp = 0
	case vm.DELEGATECALL:
		gp = 0
	}
	return gp
}

func getStaticGasPrices(revision tosca.Revision) *opCodePropertyMap[tosca.Gas] {
	if revision >= tosca.R09_Berlin {
		return &static_gas_prices_berlin
	}
	return &static_gas_prices
}

func getStaticGasPriceInternal(op vm.OpCode) tosca.Gas {
	if vm.PUSH1 <= op && op <= vm.PUSH32 {
		return 3
	}
	if vm.DUP1 <= op && op <= vm.DUP16 {
		return 3
	}
	if vm.SWAP1 <= op && op <= vm.SWAP16 {
		return 3
	}
	// this range covers: LT, GT, SLT, SGT, EQ, ISZERO,
	// AND, OR, XOR, NOT, BYTE, SHL, SHR, SAR
	if vm.LT <= op && op <= vm.SAR {
		return 3
	}
	// this range covers: COINBASE, TIMESTAMP, NUMBER,
	// DIFFICULTY/PREVRANDO, GAS, GASLIMIT, CHAINID
	if vm.COINBASE <= op && op <= vm.CHAINID {
		return 2
	}
	switch op {
	case vm.CLZ:
		return 5
	case vm.POP:
		return 2
	case vm.PUSH0:
		return 2
	case vm.ADD:
		return 3
	case vm.SUB:
		return 3
	case vm.MUL:
		return 5
	case vm.DIV:
		return 5
	case vm.SDIV:
		return 5
	case vm.MOD:
		return 5
	case vm.SMOD:
		return 5
	case vm.ADDMOD:
		return 8
	case vm.MULMOD:
		return 8
	case vm.EXP:
		return 10
	case vm.SIGNEXTEND:
		return 5
	case vm.SHA3:
		return 30
	case vm.ADDRESS:
		return 2
	case vm.BALANCE:
		return 700 // Should be 100 for warm access, 2600 for cold access
	case vm.ORIGIN:
		return 2
	case vm.CALLER:
		return 2
	case vm.CALLVALUE:
		return 2
	case vm.CALLDATALOAD:
		return 3
	case vm.CALLDATASIZE:
		return 2
	case vm.CALLDATACOPY:
		return 3
	case vm.CODESIZE:
		return 2
	case vm.CODECOPY:
		return 3
	case vm.GASPRICE:
		return 2
	case vm.EXTCODESIZE:
		return 700 // This seems to be different than documented on evm.codes (it should be 100)
	case vm.EXTCODECOPY:
		return 700 // From EIP150 it is 700, was 20
	case vm.RETURNDATASIZE:
		return 2
	case vm.RETURNDATACOPY:
		return 3
	case vm.EXTCODEHASH:
		return 700 // Should be 100 for warm access, 2600 for cold access
	case vm.BLOCKHASH:
		return 20
	case vm.SELFBALANCE:
		return 5
	case vm.BASEFEE:
		return 2
	case vm.BLOBHASH:
		return 3
	case vm.BLOBBASEFEE:
		return 2
	case vm.MLOAD:
		return 3
	case vm.MSTORE:
		return 3
	case vm.MSTORE8:
		return 3
	case vm.SLOAD:
		return 800 // This is supposed to be 100 for warm and 2100 for cold accesses
	case vm.SSTORE:
		return 0 // Costs are handled in gasSStore(..) function below
	case vm.JUMP:
		return 8
	case vm.JUMPI:
		return 10
	case vm.JUMPDEST:
		return 1
	case vm.TLOAD:
		return 100
	case vm.TSTORE:
		return 100
	case vm.PC:
		return 2
	case vm.MSIZE:
		return 2
	case vm.MCOPY:
		return 3
	case vm.GAS:
		return 2
	case vm.LOG0:
		return 375
	case vm.LOG1:
		return 750
	case vm.LOG2:
		return 1125
	case vm.LOG3:
		return 1500
	case vm.LOG4:
		return 1875
	case vm.CREATE:
		return 32000
	case vm.CREATE2:
		return 32000
	case vm.CALL:
		return 700
	case vm.CALLCODE:
		return 700
	case vm.STATICCALL:
		return 700
	case vm.RETURN:
		return 0
	case vm.STOP:
		return 0
	case vm.REVERT:
		return 0
	case vm.INVALID:
		return 0
	case vm.DELEGATECALL:
		return 700
	case vm.SELFDESTRUCT:
		return 5000
	}

	return UNKNOWN_GAS_PRICE
}

func getDynamicCostsForSstore(
	revision tosca.Revision,
	storageStatus tosca.StorageStatus,
) tosca.Gas {
	switch storageStatus {
	case tosca.StorageAdded:
		return 20000
	case tosca.StorageModified,
		tosca.StorageDeleted:
		if revision >= tosca.R09_Berlin {
			return 2900
		} else {
			return 5000
		}
	default:
		if revision >= tosca.R09_Berlin {
			return 100
		}
		return 800
	}
}

func getRefundForSstore(
	revision tosca.Revision,
	storageStatus tosca.StorageStatus,
) tosca.Gas {
	switch storageStatus {
	case tosca.StorageDeleted,
		tosca.StorageModifiedDeleted:
		if revision >= tosca.R10_London {
			return 4800
		}
		return 15000
	case tosca.StorageDeletedAdded:
		if revision >= tosca.R10_London {
			return -4800
		}
		return -15000
	case tosca.StorageDeletedRestored:
		if revision >= tosca.R10_London {
			return -4800 + 5000 - 2100 - 100
		} else if revision >= tosca.R09_Berlin {
			return -15000 + 5000 - 2100 - 100
		}
		return -15000 + 4200
	case tosca.StorageAddedDeleted:
		if revision >= tosca.R09_Berlin {
			return 19900
		}
		return 19200
	case tosca.StorageModifiedRestored:
		if revision >= tosca.R09_Berlin {
			return 5000 - 2100 - 100
		}
		return 4200
	default:
		return 0
	}
}
