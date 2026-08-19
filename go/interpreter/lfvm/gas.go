// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package lfvm

import (
	"github.com/0xsoniclabs/tosca/go/tosca"
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

	// The prices of EIP-8038, which reprices every access to durable state and
	// splits a write into an access and a write component.
	ColdAccountAccessCostAmsterdam tosca.Gas = 3000  // Cost of the first touch of an account
	ColdStorageAccessCostAmsterdam tosca.Gas = 3000  // Cost of the first touch of a storage slot
	AccountWriteCostAmsterdam      tosca.Gas = 8000  // Surcharge for the first write to an account
	StorageWriteCostAmsterdam      tosca.Gas = 10000 // Surcharge for the first write to a storage slot
	StorageClearRefundAmsterdam    tosca.Gas = 12480 // Refund for clearing a storage slot
	CreateAccessCostAmsterdam      tosca.Gas = 11000 // Constant cost of CREATE and CREATE2

	// The prices of EIP-8037, which charges the durable growth of the state in a
	// second gas dimension, priced by the size of the state entry it adds.
	CostPerStateByteAmsterdam         tosca.Gas = 1530
	AccountCreationStateCostAmsterdam tosca.Gas = 120 * CostPerStateByteAmsterdam
	StorageCreationStateCostAmsterdam tosca.Gas = 64 * CostPerStateByteAmsterdam

	UNKNOWN_GAS_PRICE = 999999
)

// getAccountAccessCost is the price of touching an account, depending on whether
// it has been touched before within the same transaction. Only relevant from
// Berlin onwards, where EIP-2929 introduced the distinction.
func getAccountAccessCost(revision tosca.Revision, accessStatus tosca.AccessStatus) tosca.Gas {
	if accessStatus == tosca.WarmAccess {
		return WarmStorageReadCostEIP2929
	}
	if revision >= tosca.R16_Amsterdam {
		return ColdAccountAccessCostAmsterdam
	}
	return ColdAccountAccessCostEIP2929
}

// getStorageAccessCost is the price of touching a storage slot, depending on
// whether it has been touched before within the same transaction. Only relevant
// from Berlin onwards, where EIP-2929 introduced the distinction.
func getStorageAccessCost(revision tosca.Revision, accessStatus tosca.AccessStatus) tosca.Gas {
	if accessStatus == tosca.WarmAccess {
		return WarmStorageReadCostEIP2929
	}
	if revision >= tosca.R16_Amsterdam {
		return ColdStorageAccessCostAmsterdam
	}
	return ColdSloadCostEIP2929
}

// getSstoreColdAccessSurcharge is what SSTORE pays on top of the price
// of a warm slot access, which is part of the costs reported by
// getDynamicCostsForSstore, when it touches the slot for the first time. Before
// Amsterdam the full cold-access price was added to the warm one, while EIP-8038
// replaced the warm price by the cold one.
func getSstoreColdAccessSurcharge(revision tosca.Revision) tosca.Gas {
	if revision >= tosca.R16_Amsterdam {
		return ColdStorageAccessCostAmsterdam - WarmStorageReadCostEIP2929
	}
	return ColdSloadCostEIP2929
}

// getCodeReadCost is what EXTCODESIZE and EXTCODECOPY pay on top of the access
// to the account for reading its code, which EIP-8038 accounts for as a second
// database lookup.
func getCodeReadCost(revision tosca.Revision) tosca.Gas {
	if revision >= tosca.R16_Amsterdam {
		return WarmStorageReadCostEIP2929
	}
	return 0
}

// getCallValueTransferCost is the price of attaching a non-zero value to a call.
// It contains the stipend granted to the callee.
func getCallValueTransferCost(revision tosca.Revision) tosca.Gas {
	if revision >= tosca.R16_Amsterdam {
		return AccountWriteCostAmsterdam + CallStipend
	}
	return CallValueTransferGas
}

// getAccountCreationStateCost is the share of creating an account charged in the
// state dimension, priced by the size of the account entry it adds to the state,
// see tosca.Gas. An operation that ends up not creating the account after all
// hands it back.
func getAccountCreationStateCost(revision tosca.Revision) tosca.Gas {
	if revision >= tosca.R16_Amsterdam {
		return AccountCreationStateCostAmsterdam
	}
	return 0
}

// getCallAccountCreationCost is what a call attaching a value pays for the empty
// account it funds. Since Amsterdam the write to the account is part of
// getCallValueTransferCost, leaving only the state dimension of the creation.
func getCallAccountCreationCost(revision tosca.Revision) (gas tosca.Gas, stateGas tosca.Gas) {
	if revision >= tosca.R16_Amsterdam {
		return 0, AccountCreationStateCostAmsterdam
	}
	return CallNewAccountGas, 0
}

// getMaxInitCodeSize returns the largest init code CREATE and CREATE2 accept.
// The limit was introduced by EIP-3860 (Shanghai) as twice the maximum size of
// deployed code and raised along with it by EIP-8038.
func getMaxInitCodeSize(revision tosca.Revision) uint64 {
	const (
		maxCodeSize          = 24576
		maxCodeSizeAmsterdam = 65536
	)
	if revision >= tosca.R16_Amsterdam {
		return 2 * maxCodeSizeAmsterdam
	}
	return 2 * maxCodeSize
}

var static_gas_prices = newOpCodePropertyMap(getStaticGasPriceInternal)
var static_gas_prices_berlin = newOpCodePropertyMap(getBerlinGasPriceInternal)
var static_gas_prices_amsterdam = newOpCodePropertyMap(getAmsterdamGasPriceInternal)

func getBerlinGasPriceInternal(op OpCode) tosca.Gas {
	gp := getStaticGasPriceInternal(op)

	// Changed static gas prices with EIP2929
	switch op {
	case SLOAD:
		gp = 0
	case EXTCODECOPY:
		gp = 0
	case EXTCODESIZE:
		gp = 0
	case EXTCODEHASH:
		gp = 0
	case BALANCE:
		gp = 0
	case CALL:
		gp = 0
	case CALLCODE:
		gp = 0
	case STATICCALL:
		gp = 0
	case DELEGATECALL:
		gp = 0
	}
	return gp
}

func getAmsterdamGasPriceInternal(op OpCode) tosca.Gas {
	gp := getBerlinGasPriceInternal(op)

	// Changed static gas prices with EIP-8038
	switch op {
	case CREATE:
		gp = CreateAccessCostAmsterdam
	case CREATE2:
		gp = CreateAccessCostAmsterdam
	}
	return gp
}

func getStaticGasPrices(revision tosca.Revision) *opCodePropertyMap[tosca.Gas] {
	if revision >= tosca.R16_Amsterdam {
		return &static_gas_prices_amsterdam
	}
	if revision >= tosca.R09_Berlin {
		return &static_gas_prices_berlin
	}
	return &static_gas_prices
}

func getStaticGasPriceInternal(op OpCode) tosca.Gas {
	if PUSH1 <= op && op <= PUSH32 {
		return 3
	}
	if DUP1 <= op && op <= DUP16 {
		return 3
	}
	if SWAP1 <= op && op <= SWAP16 {
		return 3
	}
	// this range covers: LT, GT, SLT, SGT, EQ, ISZERO,
	// AND, OR, XOR, NOT, BYTE, SHL, SHR, SAR
	if LT <= op && op <= SAR {
		return 3
	}
	// this range covers: COINBASE, TIMESTAMP, NUMBER,
	// DIFFICULTY/PREVRANDO, GAS, GASLIMIT, CHAINID
	if COINBASE <= op && op <= CHAINID {
		return 2
	}
	switch op {
	case CLZ:
		return 5
	case POP:
		return 2
	case PUSH0:
		return 2
	case ADD:
		return 3
	case SUB:
		return 3
	case MUL:
		return 5
	case DIV:
		return 5
	case SDIV:
		return 5
	case MOD:
		return 5
	case SMOD:
		return 5
	case ADDMOD:
		return 8
	case MULMOD:
		return 8
	case EXP:
		return 10
	case SIGNEXTEND:
		return 5
	case SHA3:
		return 30
	case ADDRESS:
		return 2
	case BALANCE:
		return 700 // Should be 100 for warm access, 2600 for cold access
	case ORIGIN:
		return 2
	case CALLER:
		return 2
	case CALLVALUE:
		return 2
	case CALLDATALOAD:
		return 3
	case CALLDATASIZE:
		return 2
	case CALLDATACOPY:
		return 3
	case CODESIZE:
		return 2
	case CODECOPY:
		return 3
	case GASPRICE:
		return 2
	case EXTCODESIZE:
		return 700 // This seems to be different than documented on evm.codes (it should be 100)
	case EXTCODECOPY:
		return 700 // From EIP150 it is 700, was 20
	case RETURNDATASIZE:
		return 2
	case RETURNDATACOPY:
		return 3
	case EXTCODEHASH:
		return 700 // Should be 100 for warm access, 2600 for cold access
	case BLOCKHASH:
		return 20
	case SELFBALANCE:
		return 5
	case BASEFEE:
		return 2
	case BLOBHASH:
		return 3
	case BLOBBASEFEE:
		return 2
	case SLOTNUM:
		return 2
	case DUPN:
		return 3
	case SWAPN:
		return 3
	case EXCHANGE:
		return 3
	case MLOAD:
		return 3
	case MSTORE:
		return 3
	case MSTORE8:
		return 3
	case SLOAD:
		return 800 // This is supposed to be 100 for warm and 2100 for cold accesses
	case SSTORE:
		return 0 // Costs are handled in gasSStore(..) function below
	case JUMP:
		return 8
	case JUMPI:
		return 10
	case JUMPDEST:
		return 1
	case JUMP_TO:
		return 0
	case TLOAD:
		return 100
	case TSTORE:
		return 100
	case PC:
		return 2
	case MSIZE:
		return 2
	case MCOPY:
		return 3
	case GAS:
		return 2
	case LOG0:
		return 375
	case LOG1:
		return 750
	case LOG2:
		return 1125
	case LOG3:
		return 1500
	case LOG4:
		return 1875
	case CREATE:
		return 32000
	case CREATE2:
		return 32000
	case CALL:
		return 700
	case CALLCODE:
		return 700
	case STATICCALL:
		return 700
	case RETURN:
		return 0
	case STOP:
		return 0
	case REVERT:
		return 0
	case INVALID:
		return 0
	case DELEGATECALL:
		return 700
	case SELFDESTRUCT:
		return 5000
	}

	if op.isSuperInstruction() {
		var sum tosca.Gas
		for _, subOp := range op.decompose() {
			sum += getStaticGasPriceInternal(subOp)
		}
		return sum
	}

	return UNKNOWN_GAS_PRICE
}

func getDynamicCostsForSstore(
	revision tosca.Revision,
	storageStatus tosca.StorageStatus,
) tosca.Gas {
	// EIP-8038 replaced the distinct prices for creating and overwriting a slot
	// by a single write surcharge on top of the access to the slot. The durable
	// growth creating a slot causes is charged in the state dimension instead,
	// see getStateCostsForSstore.
	if revision >= tosca.R16_Amsterdam {
		switch storageStatus {
		case tosca.StorageAdded,
			tosca.StorageModified,
			tosca.StorageDeleted:
			return WarmStorageReadCostEIP2929 + StorageWriteCostAmsterdam
		default:
			return WarmStorageReadCostEIP2929
		}
	}

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

// getStateCostsForSstore returns the gas an SSTORE charges in the state
// dimension for the durable growth of creating a storage slot, and the part of
// such an earlier charge it hands back by clearing the slot again within the
// same transaction, see tosca.Gas.
func getStateCostsForSstore(
	revision tosca.Revision,
	storageStatus tosca.StorageStatus,
) (charge tosca.Gas, refund tosca.Gas) {
	if revision < tosca.R16_Amsterdam {
		return 0, 0
	}
	switch storageStatus {
	case tosca.StorageAdded:
		return StorageCreationStateCostAmsterdam, 0
	case tosca.StorageAddedDeleted:
		return 0, StorageCreationStateCostAmsterdam
	}
	return 0, 0
}

func getRefundForSstore(
	revision tosca.Revision,
	storageStatus tosca.StorageStatus,
) tosca.Gas {
	// EIP-8038 reprices the refund for clearing a slot and grants back the write
	// surcharge whenever a slot ends up holding its committed value again.
	if revision >= tosca.R16_Amsterdam {
		switch storageStatus {
		case tosca.StorageDeleted,
			tosca.StorageModifiedDeleted:
			return StorageClearRefundAmsterdam
		case tosca.StorageDeletedAdded:
			return -StorageClearRefundAmsterdam
		case tosca.StorageDeletedRestored:
			return StorageWriteCostAmsterdam - StorageClearRefundAmsterdam
		case tosca.StorageAddedDeleted,
			tosca.StorageModifiedRestored:
			return StorageWriteCostAmsterdam
		default:
			return 0
		}
	}

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
