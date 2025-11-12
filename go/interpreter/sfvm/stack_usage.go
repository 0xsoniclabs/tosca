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
func computeStackUsage(op OpCode) stackUsage {

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

	if PUSH1 <= op && op <= PUSH32 {
		return makeUsage(0, 1)
	}
	if DUP1 <= op && op <= DUP16 {
		return makeUsage(int(op-DUP1+1), int(op-DUP1+2))
	}
	if SWAP1 <= op && op <= SWAP16 {
		return makeUsage(int(op-SWAP1+2), int(op-SWAP1+2))
	}
	if LOG0 <= op && op <= LOG4 {
		return makeUsage(int(op-LOG0+2), 0)
	}

	switch op {
	case JUMPDEST, JUMP_TO, STOP:
		return makeUsage(0, 0)
	case PUSH0, MSIZE, ADDRESS, ORIGIN, CALLER, CALLVALUE, CALLDATASIZE,
		CODESIZE, GASPRICE, COINBASE, TIMESTAMP, NUMBER,
		PREVRANDAO, GASLIMIT, PC, GAS, RETURNDATASIZE,
		SELFBALANCE, CHAINID, BASEFEE, BLOBBASEFEE:
		return makeUsage(0, 1)
	case POP, JUMP, SELFDESTRUCT:
		return makeUsage(1, 0)
	case ISZERO, NOT, BALANCE, CALLDATALOAD, EXTCODESIZE,
		BLOCKHASH, MLOAD, SLOAD, TLOAD, EXTCODEHASH, BLOBHASH, CLZ:
		return makeUsage(1, 1)
	case MSTORE, MSTORE8, SSTORE, TSTORE, JUMPI, RETURN, REVERT:
		return makeUsage(2, 0)
	case ADD, SUB, MUL, DIV, SDIV, MOD, SMOD, EXP, SIGNEXTEND,
		SHA3, LT, GT, SLT, SGT, EQ, AND, XOR, OR, BYTE,
		SHL, SHR, SAR:
		return makeUsage(2, 1)
	case CALLDATACOPY, CODECOPY, RETURNDATACOPY, MCOPY:
		return makeUsage(3, 0)
	case ADDMOD, MULMOD, CREATE:
		return makeUsage(3, 1)
	case EXTCODECOPY:
		return makeUsage(4, 0)
	case CREATE2:
		return makeUsage(4, 1)
	case STATICCALL, DELEGATECALL:
		return makeUsage(6, 1)
	case CALL, CALLCODE:
		return makeUsage(7, 1)
	}

	return stackUsage{}
}
