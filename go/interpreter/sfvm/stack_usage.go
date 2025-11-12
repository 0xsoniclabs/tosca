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

import "github.com/0xsoniclabs/tosca/go/tosca/vm"

// stackUsage defines the combined effect of an instruction on the stack. Each
// instruction is accessing a range of elements on the stack relative to the
// stack pointer. The range is given by the interval [from, to) where from is
// the lower end and to is the upper end of the accessed interval. The delta
// field represents the change in the stack size caused by the instruction.
type stackUsage struct {
	from, to, delta int
}

// computeStackUsage computes the stack usage of the given opcode. The result
// is a stackUsage struct that defines the combined effect of the instruction
// on the stack. If the opcode is not known, zero stack usage is reported.
func computeStackUsage(op vm.OpCode) stackUsage {

	// For single instructions it is easiest to define the stack usage based on
	// the opcode's pops and pushes.
	makeUsage := func(pops, pushes int) stackUsage {
		delta := pushes - pops
		to := 0
		if delta > 0 {
			to = delta
		}
		return stackUsage{from: -pops, to: to, delta: delta}
	}

	if vm.PUSH1 <= op && op <= vm.PUSH32 {
		return makeUsage(0, 1)
	}
	if vm.DUP1 <= op && op <= vm.DUP16 {
		return makeUsage(int(op-vm.DUP1+1), int(op-vm.DUP1+2))
	}
	if vm.SWAP1 <= op && op <= vm.SWAP16 {
		return makeUsage(int(op-vm.SWAP1+2), int(op-vm.SWAP1+2))
	}
	if vm.LOG0 <= op && op <= vm.LOG4 {
		return makeUsage(int(op-vm.LOG0+2), 0)
	}

	switch op {
	case vm.JUMPDEST, vm.STOP:
		return makeUsage(0, 0)
	case vm.PUSH0, vm.MSIZE, vm.ADDRESS, vm.ORIGIN, vm.CALLER, vm.CALLVALUE, vm.CALLDATASIZE,
		vm.CODESIZE, vm.GASPRICE, vm.COINBASE, vm.TIMESTAMP, vm.NUMBER,
		vm.PREVRANDAO, vm.GASLIMIT, vm.PC, vm.GAS, vm.RETURNDATASIZE,
		vm.SELFBALANCE, vm.CHAINID, vm.BASEFEE, vm.BLOBBASEFEE:
		return makeUsage(0, 1)
	case vm.POP, vm.JUMP, vm.SELFDESTRUCT:
		return makeUsage(1, 0)
	case vm.ISZERO, vm.NOT, vm.BALANCE, vm.CALLDATALOAD, vm.EXTCODESIZE,
		vm.BLOCKHASH, vm.MLOAD, vm.SLOAD, vm.TLOAD, vm.EXTCODEHASH, vm.BLOBHASH, vm.CLZ:
		return makeUsage(1, 1)
	case vm.MSTORE, vm.MSTORE8, vm.SSTORE, vm.TSTORE, vm.JUMPI, vm.RETURN, vm.REVERT:
		return makeUsage(2, 0)
	case vm.ADD, vm.SUB, vm.MUL, vm.DIV, vm.SDIV, vm.MOD, vm.SMOD, vm.EXP, vm.SIGNEXTEND,
		vm.SHA3, vm.LT, vm.GT, vm.SLT, vm.SGT, vm.EQ, vm.AND, vm.XOR, vm.OR, vm.BYTE,
		vm.SHL, vm.SHR, vm.SAR:
		return makeUsage(2, 1)
	case vm.CALLDATACOPY, vm.CODECOPY, vm.RETURNDATACOPY, vm.MCOPY:
		return makeUsage(3, 0)
	case vm.ADDMOD, vm.MULMOD, vm.CREATE:
		return makeUsage(3, 1)
	case vm.EXTCODECOPY:
		return makeUsage(4, 0)
	case vm.CREATE2:
		return makeUsage(4, 1)
	case vm.STATICCALL, vm.DELEGATECALL:
		return makeUsage(6, 1)
	case vm.CALL, vm.CALLCODE:
		return makeUsage(7, 1)
	}

	return stackUsage{}
}
