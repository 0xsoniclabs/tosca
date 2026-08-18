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

// This file implements the decoding of the immediate operands introduced by
// EIP-8024 (https://eips.ethereum.org/EIPS/eip-8024).
//
// Unlike the data of a PUSH instruction, such an operand is not excluded from
// jumpdest analysis. The encodings below exclude every byte that is a JUMPDEST
// (0x5b) or a PUSH (0x5f-0x7f) precisely so that the analysis can stay
// unchanged: `e6 5b` keeps decoding to INVALID followed by a valid JUMPDEST.

// HasImmediateOperand reports whether the given instruction reads the byte
// following it as an operand.
func HasImmediateOperand(op OpCode) bool {
	return op == DUPN || op == SWAPN || op == EXCHANGE
}

// IsValidImmediateOperand reports whether x may be used as the immediate
// operand of op. Executing op with any other operand fails.
func IsValidImmediateOperand(op OpCode, x byte) bool {
	switch op {
	case DUPN, SWAPN:
		return x <= 90 || 128 <= x
	case EXCHANGE:
		return x <= 81 || 128 <= x
	}
	return false
}

// DecodeSingleImmediate decodes the operand of DUPN and SWAPN into the stack
// depth it addresses, a value in [17, 235]. The result is only meaningful for
// operands accepted by IsValidImmediateOperand.
func DecodeSingleImmediate(x byte) int {
	return (int(x) + 145) % 256
}

// DecodePairImmediate decodes the operand of EXCHANGE into the pair of stack
// positions it addresses, with 1 <= n < m and n+m <= 30. The result is only
// meaningful for operands accepted by IsValidImmediateOperand.
func DecodePairImmediate(x byte) (n int, m int) {
	// The XOR moves the excluded byte range out of the 210 encodable positions
	// of a 16x16 grid, of which the upper triangle holds the pairs with m <= 16
	// and the lower triangle the pairs with m > 16.
	q, r := int(x^143)/16, int(x^143)%16
	if q < r {
		return q + 1, r + 1
	}
	return r + 1, 29 - q
}

// MinStackSizeForImmediate returns the number of stack elements op requires to
// be executed with the immediate operand x. The result is only meaningful for
// operands accepted by IsValidImmediateOperand.
func MinStackSizeForImmediate(op OpCode, x byte) int {
	switch op {
	case DUPN:
		return DecodeSingleImmediate(x)
	case SWAPN:
		return DecodeSingleImmediate(x) + 1
	case EXCHANGE:
		_, m := DecodePairImmediate(x)
		return m + 1
	}
	return 0
}
