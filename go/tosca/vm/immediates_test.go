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

import "testing"

func TestImmediates_OnlyEip8024InstructionsHaveAnOperand(t *testing.T) {
	withOperand := map[OpCode]bool{DUPN: true, SWAPN: true, EXCHANGE: true}
	for i := 0; i < 256; i++ {
		op := OpCode(i)
		if got, want := HasImmediateOperand(op), withOperand[op]; got != want {
			t.Errorf("HasImmediateOperand(%v) = %t, want %t", op, got, want)
		}
	}
}

// TestImmediates_ExcludedOperandsCannotDisturbJumpdestAnalysis covers the
// property EIP-8024 is built on: an operand is never a JUMPDEST and never a
// PUSH, so leaving jumpdest analysis unchanged stays correct.
func TestImmediates_ExcludedOperandsCannotDisturbJumpdestAnalysis(t *testing.T) {
	for _, op := range []OpCode{DUPN, SWAPN, EXCHANGE} {
		for i := 0; i < 256; i++ {
			operand := OpCode(i)
			if !IsValidImmediateOperand(op, byte(i)) {
				continue
			}
			if operand == JUMPDEST || (PUSH0 <= operand && operand <= PUSH32) {
				t.Errorf("%v accepts %v as operand", op, operand)
			}
		}
	}
}

func TestImmediates_SingleOperandsDecodeToDistinctDepths(t *testing.T) {
	seen := map[int]bool{}
	for i := 0; i < 256; i++ {
		if valid := IsValidImmediateOperand(DUPN, byte(i)); !valid {
			if 90 < i && i < 128 {
				continue
			}
			t.Fatalf("operand 0x%02x should be valid", i)
		}
		n := DecodeSingleImmediate(byte(i))
		if n < 17 || n > 235 {
			t.Errorf("operand 0x%02x decodes to %d, outside [17, 235]", i, n)
		}
		if seen[n] {
			t.Errorf("depth %d is encoded by more than one operand", n)
		}
		seen[n] = true
	}
	if got, want := len(seen), 219; got != want {
		t.Errorf("got %d distinct depths, want %d", got, want)
	}
}

func TestImmediates_PairOperandsDecodeToDistinctPositionPairs(t *testing.T) {
	type pair struct{ n, m int }
	seen := map[pair]bool{}
	for i := 0; i < 256; i++ {
		if valid := IsValidImmediateOperand(EXCHANGE, byte(i)); !valid {
			if 81 < i && i < 128 {
				continue
			}
			t.Fatalf("operand 0x%02x should be valid", i)
		}
		n, m := DecodePairImmediate(byte(i))
		if !(1 <= n && n < m && n+m <= 30) {
			t.Errorf("operand 0x%02x decodes to (%d, %d), violating 1 <= n < m and n+m <= 30", i, n, m)
		}
		if seen[pair{n, m}] {
			t.Errorf("pair (%d, %d) is encoded by more than one operand", n, m)
		}
		seen[pair{n, m}] = true
	}
	if got, want := len(seen), 210; got != want {
		t.Errorf("got %d distinct pairs, want %d", got, want)
	}
}

func TestImmediates_MinStackSizeCoversAllAddressedPositions(t *testing.T) {
	for i := 0; i < 256; i++ {
		operand := byte(i)
		if IsValidImmediateOperand(DUPN, operand) {
			n := DecodeSingleImmediate(operand)
			if got, want := MinStackSizeForImmediate(DUPN, operand), n; got != want {
				t.Errorf("DUPN 0x%02x requires %d elements, want %d", i, got, want)
			}
			if got, want := MinStackSizeForImmediate(SWAPN, operand), n+1; got != want {
				t.Errorf("SWAPN 0x%02x requires %d elements, want %d", i, got, want)
			}
		}
		if IsValidImmediateOperand(EXCHANGE, operand) {
			_, m := DecodePairImmediate(operand)
			if got, want := MinStackSizeForImmediate(EXCHANGE, operand), m+1; got != want {
				t.Errorf("EXCHANGE 0x%02x requires %d elements, want %d", i, got, want)
			}
		}
	}
	if got := MinStackSizeForImmediate(ADD, 0); got != 0 {
		t.Errorf("instruction without operand requires %d elements, want 0", got)
	}
}
