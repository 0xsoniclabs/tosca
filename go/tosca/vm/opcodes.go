// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package vm

import (
	"fmt"
)

type OpCode byte

const (
	STOP           OpCode = 0x00
	ADD            OpCode = 0x01
	MUL            OpCode = 0x02
	SUB            OpCode = 0x03
	DIV            OpCode = 0x04
	SDIV           OpCode = 0x05
	MOD            OpCode = 0x06
	SMOD           OpCode = 0x07
	ADDMOD         OpCode = 0x08
	MULMOD         OpCode = 0x09
	EXP            OpCode = 0x0A
	SIGNEXTEND     OpCode = 0x0B
	LT             OpCode = 0x10
	GT             OpCode = 0x11
	SLT            OpCode = 0x12
	SGT            OpCode = 0x13
	EQ             OpCode = 0x14
	ISZERO         OpCode = 0x15
	AND            OpCode = 0x16
	OR             OpCode = 0x17
	XOR            OpCode = 0x18
	NOT            OpCode = 0x19
	BYTE           OpCode = 0x1A
	SHL            OpCode = 0x1B
	SHR            OpCode = 0x1C
	SAR            OpCode = 0x1D
	SHA3           OpCode = 0x20
	ADDRESS        OpCode = 0x30
	BALANCE        OpCode = 0x31
	ORIGIN         OpCode = 0x32
	CALLER         OpCode = 0x33
	CALLVALUE      OpCode = 0x34
	CALLDATALOAD   OpCode = 0x35
	CALLDATASIZE   OpCode = 0x36
	CALLDATACOPY   OpCode = 0x37
	CODESIZE       OpCode = 0x38
	CODECOPY       OpCode = 0x39
	GASPRICE       OpCode = 0x3A
	EXTCODESIZE    OpCode = 0x3B
	EXTCODECOPY    OpCode = 0x3C
	RETURNDATASIZE OpCode = 0x3D
	RETURNDATACOPY OpCode = 0x3E
	EXTCODEHASH    OpCode = 0x3F
	BLOCKHASH      OpCode = 0x40
	COINBASE       OpCode = 0x41
	TIMESTAMP      OpCode = 0x42
	NUMBER         OpCode = 0x43
	PREVRANDAO     OpCode = 0x44
	GASLIMIT       OpCode = 0x45
	CHAINID        OpCode = 0x46
	SELFBALANCE    OpCode = 0x47
	BASEFEE        OpCode = 0x48
	BLOBHASH       OpCode = 0x49
	BLOBBASEFEE    OpCode = 0x4A
	POP            OpCode = 0x50
	MLOAD          OpCode = 0x51
	MSTORE         OpCode = 0x52
	MSTORE8        OpCode = 0x53
	SLOAD          OpCode = 0x54
	SSTORE         OpCode = 0x55
	JUMP           OpCode = 0x56
	JUMPI          OpCode = 0x57
	PC             OpCode = 0x58
	MSIZE          OpCode = 0x59
	GAS            OpCode = 0x5A
	JUMPDEST       OpCode = 0x5B
	TLOAD          OpCode = 0x5C
	TSTORE         OpCode = 0x5D
	PUSH0          OpCode = 0x5F
	MCOPY          OpCode = 0x5E
	PUSH1          OpCode = 0x60
	PUSH2          OpCode = 0x61
	PUSH3          OpCode = 0x62
	PUSH4          OpCode = 0x63
	PUSH5          OpCode = 0x64
	PUSH6          OpCode = 0x65
	PUSH7          OpCode = 0x66
	PUSH8          OpCode = 0x67
	PUSH9          OpCode = 0x68
	PUSH10         OpCode = 0x69
	PUSH11         OpCode = 0x6A
	PUSH12         OpCode = 0x6B
	PUSH13         OpCode = 0x6C
	PUSH14         OpCode = 0x6D
	PUSH15         OpCode = 0x6E
	PUSH16         OpCode = 0x6F
	PUSH17         OpCode = 0x70
	PUSH18         OpCode = 0x71
	PUSH19         OpCode = 0x72
	PUSH20         OpCode = 0x73
	PUSH21         OpCode = 0x74
	PUSH22         OpCode = 0x75
	PUSH23         OpCode = 0x76
	PUSH24         OpCode = 0x77
	PUSH25         OpCode = 0x78
	PUSH26         OpCode = 0x79
	PUSH27         OpCode = 0x7A
	PUSH28         OpCode = 0x7B
	PUSH29         OpCode = 0x7C
	PUSH30         OpCode = 0x7D
	PUSH31         OpCode = 0x7E
	PUSH32         OpCode = 0x7F
	DUP1           OpCode = 0x80
	DUP2           OpCode = 0x81
	DUP3           OpCode = 0x82
	DUP4           OpCode = 0x83
	DUP5           OpCode = 0x84
	DUP6           OpCode = 0x85
	DUP7           OpCode = 0x86
	DUP8           OpCode = 0x87
	DUP9           OpCode = 0x88
	DUP10          OpCode = 0x89
	DUP11          OpCode = 0x8A
	DUP12          OpCode = 0x8B
	DUP13          OpCode = 0x8C
	DUP14          OpCode = 0x8D
	DUP15          OpCode = 0x8E
	DUP16          OpCode = 0x8F
	SWAP1          OpCode = 0x90
	SWAP2          OpCode = 0x91
	SWAP3          OpCode = 0x92
	SWAP4          OpCode = 0x93
	SWAP5          OpCode = 0x94
	SWAP6          OpCode = 0x95
	SWAP7          OpCode = 0x96
	SWAP8          OpCode = 0x97
	SWAP9          OpCode = 0x98
	SWAP10         OpCode = 0x99
	SWAP11         OpCode = 0x9A
	SWAP12         OpCode = 0x9B
	SWAP13         OpCode = 0x9C
	SWAP14         OpCode = 0x9D
	SWAP15         OpCode = 0x9E
	SWAP16         OpCode = 0x9F
	LOG0           OpCode = 0xA0
	LOG1           OpCode = 0xA1
	LOG2           OpCode = 0xA2
	LOG3           OpCode = 0xA3
	LOG4           OpCode = 0xA4
	CREATE         OpCode = 0xF0
	CALL           OpCode = 0xF1
	CALLCODE       OpCode = 0xF2
	RETURN         OpCode = 0xF3
	DELEGATECALL   OpCode = 0xF4
	CREATE2        OpCode = 0xF5
	STATICCALL     OpCode = 0xFA
	REVERT         OpCode = 0xFD
	INVALID        OpCode = 0xFE
	SELFDESTRUCT   OpCode = 0xFF
)

func (op OpCode) Width() int {
	if PUSH1 <= op && op <= PUSH32 {
		return int(op-PUSH1) + 2
	} else {
		return 1
	}
}

// ValidOpCodesNoPush returns a slice of valid op codes, but no PUSH instruction.
func ValidOpCodesNoPush() []OpCode {
	res := make([]OpCode, 0, 256)
	for i := 0; i < 256; i++ {
		op := OpCode(i)
		if PUSH0 <= op && op <= PUSH32 {
			continue
		}
		if IsValid(op) {
			res = append(res, op)
		}
	}
	return res
}

func (op OpCode) String() string {
	switch op {
	case STOP:
		return "STOP"
	case ADD:
		return "ADD"
	case MUL:
		return "MUL"
	case SUB:
		return "SUB"
	case DIV:
		return "DIV"
	case SDIV:
		return "SDIV"
	case MOD:
		return "MOD"
	case SMOD:
		return "SMOD"
	case ADDMOD:
		return "ADDMOD"
	case MULMOD:
		return "MULMOD"
	case EXP:
		return "EXP"
	case SIGNEXTEND:
		return "SIGNEXTEND"
	case LT:
		return "LT"
	case GT:
		return "GT"
	case SLT:
		return "SLT"
	case SGT:
		return "SGT"
	case EQ:
		return "EQ"
	case ISZERO:
		return "ISZERO"
	case AND:
		return "AND"
	case OR:
		return "OR"
	case XOR:
		return "XOR"
	case NOT:
		return "NOT"
	case BYTE:
		return "BYTE"
	case SHL:
		return "SHL"
	case SHR:
		return "SHR"
	case SAR:
		return "SAR"
	case SHA3:
		return "SHA3"
	case ADDRESS:
		return "ADDRESS"
	case BALANCE:
		return "BALANCE"
	case ORIGIN:
		return "ORIGIN"
	case CALLER:
		return "CALLER"
	case CALLVALUE:
		return "CALLVALUE"
	case CALLDATALOAD:
		return "CALLDATALOAD"
	case CALLDATASIZE:
		return "CALLDATASIZE"
	case CALLDATACOPY:
		return "CALLDATACOPY"
	case CODESIZE:
		return "CODESIZE"
	case CODECOPY:
		return "CODECOPY"
	case GASPRICE:
		return "GASPRICE"
	case EXTCODESIZE:
		return "EXTCODESIZE"
	case EXTCODECOPY:
		return "EXTCODECOPY"
	case RETURNDATASIZE:
		return "RETURNDATASIZE"
	case RETURNDATACOPY:
		return "RETURNDATACOPY"
	case EXTCODEHASH:
		return "EXTCODEHASH"
	case BLOCKHASH:
		return "BLOCKHASH"
	case COINBASE:
		return "COINBASE"
	case TIMESTAMP:
		return "TIMESTAMP"
	case NUMBER:
		return "NUMBER"
	case PREVRANDAO:
		return "PREVRANDAO"
	case GASLIMIT:
		return "GASLIMIT"
	case CHAINID:
		return "CHAINID"
	case SELFBALANCE:
		return "SELFBALANCE"
	case BASEFEE:
		return "BASEFEE"
	case BLOBHASH:
		return "BLOBHASH"
	case BLOBBASEFEE:
		return "BLOBBASEFEE"
	case POP:
		return "POP"
	case MLOAD:
		return "MLOAD"
	case MSTORE:
		return "MSTORE"
	case MSTORE8:
		return "MSTORE8"
	case SLOAD:
		return "SLOAD"
	case SSTORE:
		return "SSTORE"
	case JUMP:
		return "JUMP"
	case JUMPI:
		return "JUMPI"
	case PC:
		return "PC"
	case MSIZE:
		return "MSIZE"
	case GAS:
		return "GAS"
	case JUMPDEST:
		return "JUMPDEST"
	case TLOAD:
		return "TLOAD"
	case TSTORE:
		return "TSTORE"
	case PUSH0:
		return "PUSH0"
	case MCOPY:
		return "MCOPY"
	case PUSH1:
		return "PUSH1"
	case PUSH2:
		return "PUSH2"
	case PUSH3:
		return "PUSH3"
	case PUSH4:
		return "PUSH4"
	case PUSH5:
		return "PUSH5"
	case PUSH6:
		return "PUSH6"
	case PUSH7:
		return "PUSH7"
	case PUSH8:
		return "PUSH8"
	case PUSH9:
		return "PUSH9"
	case PUSH10:
		return "PUSH10"
	case PUSH11:
		return "PUSH11"
	case PUSH12:
		return "PUSH12"
	case PUSH13:
		return "PUSH13"
	case PUSH14:
		return "PUSH14"
	case PUSH15:
		return "PUSH15"
	case PUSH16:
		return "PUSH16"
	case PUSH17:
		return "PUSH17"
	case PUSH18:
		return "PUSH18"
	case PUSH19:
		return "PUSH19"
	case PUSH20:
		return "PUSH20"
	case PUSH21:
		return "PUSH21"
	case PUSH22:
		return "PUSH22"
	case PUSH23:
		return "PUSH23"
	case PUSH24:
		return "PUSH24"
	case PUSH25:
		return "PUSH25"
	case PUSH26:
		return "PUSH26"
	case PUSH27:
		return "PUSH27"
	case PUSH28:
		return "PUSH28"
	case PUSH29:
		return "PUSH29"
	case PUSH30:
		return "PUSH30"
	case PUSH31:
		return "PUSH31"
	case PUSH32:
		return "PUSH32"
	case DUP1:
		return "DUP1"
	case DUP2:
		return "DUP2"
	case DUP3:
		return "DUP3"
	case DUP4:
		return "DUP4"
	case DUP5:
		return "DUP5"
	case DUP6:
		return "DUP6"
	case DUP7:
		return "DUP7"
	case DUP8:
		return "DUP8"
	case DUP9:
		return "DUP9"
	case DUP10:
		return "DUP10"
	case DUP11:
		return "DUP11"
	case DUP12:
		return "DUP12"
	case DUP13:
		return "DUP13"
	case DUP14:
		return "DUP14"
	case DUP15:
		return "DUP15"
	case DUP16:
		return "DUP16"
	case SWAP1:
		return "SWAP1"
	case SWAP2:
		return "SWAP2"
	case SWAP3:
		return "SWAP3"
	case SWAP4:
		return "SWAP4"
	case SWAP5:
		return "SWAP5"
	case SWAP6:
		return "SWAP6"
	case SWAP7:
		return "SWAP7"
	case SWAP8:
		return "SWAP8"
	case SWAP9:
		return "SWAP9"
	case SWAP10:
		return "SWAP10"
	case SWAP11:
		return "SWAP11"
	case SWAP12:
		return "SWAP12"
	case SWAP13:
		return "SWAP13"
	case SWAP14:
		return "SWAP14"
	case SWAP15:
		return "SWAP15"
	case SWAP16:
		return "SWAP16"
	case LOG0:
		return "LOG0"
	case LOG1:
		return "LOG1"
	case LOG2:
		return "LOG2"
	case LOG3:
		return "LOG3"
	case LOG4:
		return "LOG4"
	case CREATE:
		return "CREATE"
	case CALL:
		return "CALL"
	case CALLCODE:
		return "CALLCODE"
	case RETURN:
		return "RETURN"
	case DELEGATECALL:
		return "DELEGATECALL"
	case CREATE2:
		return "CREATE2"
	case STATICCALL:
		return "STATICCALL"
	case REVERT:
		return "REVERT"
	case INVALID:
		return "INVALID"
	case SELFDESTRUCT:
		return "SELFDESTRUCT"
	default:
		return fmt.Sprintf("op(0x%02X)", byte(op))
	}
}

// IsValid determines whether the given OpCode is a valid operation
// for any revision.
func IsValid(op OpCode) bool {
	return _validOpCodes[op]
}

var _validOpCodes = initValidOpCodes()

func initValidOpCodes() [256]bool {
	res := [256]bool{}
	res[STOP] = true
	res[ADD] = true
	res[MUL] = true
	res[SUB] = true
	res[DIV] = true
	res[SDIV] = true
	res[MOD] = true
	res[SMOD] = true
	res[ADDMOD] = true
	res[MULMOD] = true
	res[EXP] = true
	res[SIGNEXTEND] = true
	res[LT] = true
	res[GT] = true
	res[SLT] = true
	res[SGT] = true
	res[EQ] = true
	res[ISZERO] = true
	res[AND] = true
	res[OR] = true
	res[XOR] = true
	res[NOT] = true
	res[BYTE] = true
	res[SHL] = true
	res[SHR] = true
	res[SAR] = true
	res[SHA3] = true
	res[ADDRESS] = true
	res[BALANCE] = true
	res[ORIGIN] = true
	res[CALLER] = true
	res[CALLVALUE] = true
	res[CALLDATALOAD] = true
	res[CALLDATASIZE] = true
	res[CALLDATACOPY] = true
	res[CODESIZE] = true
	res[CODECOPY] = true
	res[GASPRICE] = true
	res[EXTCODESIZE] = true
	res[EXTCODECOPY] = true
	res[RETURNDATASIZE] = true
	res[RETURNDATACOPY] = true
	res[EXTCODEHASH] = true
	res[BLOCKHASH] = true
	res[COINBASE] = true
	res[TIMESTAMP] = true
	res[NUMBER] = true
	res[PREVRANDAO] = true
	res[GASLIMIT] = true
	res[CHAINID] = true
	res[SELFBALANCE] = true
	res[BASEFEE] = true
	res[BLOBHASH] = true
	res[BLOBBASEFEE] = true
	res[POP] = true
	res[MLOAD] = true
	res[MSTORE] = true
	res[MSTORE8] = true
	res[SLOAD] = true
	res[SSTORE] = true
	res[JUMP] = true
	res[JUMPI] = true
	res[PC] = true
	res[MSIZE] = true
	res[GAS] = true
	res[JUMPDEST] = true
	res[TLOAD] = true
	res[TSTORE] = true
	res[PUSH0] = true
	res[MCOPY] = true
	res[PUSH1] = true
	res[PUSH2] = true
	res[PUSH3] = true
	res[PUSH4] = true
	res[PUSH5] = true
	res[PUSH6] = true
	res[PUSH7] = true
	res[PUSH8] = true
	res[PUSH9] = true
	res[PUSH10] = true
	res[PUSH11] = true
	res[PUSH12] = true
	res[PUSH13] = true
	res[PUSH14] = true
	res[PUSH15] = true
	res[PUSH16] = true
	res[PUSH17] = true
	res[PUSH18] = true
	res[PUSH19] = true
	res[PUSH20] = true
	res[PUSH21] = true
	res[PUSH22] = true
	res[PUSH23] = true
	res[PUSH24] = true
	res[PUSH25] = true
	res[PUSH26] = true
	res[PUSH27] = true
	res[PUSH28] = true
	res[PUSH29] = true
	res[PUSH30] = true
	res[PUSH31] = true
	res[PUSH32] = true
	res[DUP1] = true
	res[DUP2] = true
	res[DUP3] = true
	res[DUP4] = true
	res[DUP5] = true
	res[DUP6] = true
	res[DUP7] = true
	res[DUP8] = true
	res[DUP9] = true
	res[DUP10] = true
	res[DUP11] = true
	res[DUP12] = true
	res[DUP13] = true
	res[DUP14] = true
	res[DUP15] = true
	res[DUP16] = true
	res[SWAP1] = true
	res[SWAP2] = true
	res[SWAP3] = true
	res[SWAP4] = true
	res[SWAP5] = true
	res[SWAP6] = true
	res[SWAP7] = true
	res[SWAP8] = true
	res[SWAP9] = true
	res[SWAP10] = true
	res[SWAP11] = true
	res[SWAP12] = true
	res[SWAP13] = true
	res[SWAP14] = true
	res[SWAP15] = true
	res[SWAP16] = true
	res[LOG0] = true
	res[LOG1] = true
	res[LOG2] = true
	res[LOG3] = true
	res[LOG4] = true
	res[CREATE] = true
	res[CALL] = true
	res[CALLCODE] = true
	res[RETURN] = true
	res[DELEGATECALL] = true
	res[CREATE2] = true
	res[STATICCALL] = true
	res[REVERT] = true
	res[SELFDESTRUCT] = true
	return res
}
