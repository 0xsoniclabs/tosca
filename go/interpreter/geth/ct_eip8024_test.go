// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package geth

import (
	"fmt"
	"testing"

	"github.com/0xsoniclabs/tosca/go/ct/common"
	"github.com/0xsoniclabs/tosca/go/ct/spc"
	"github.com/0xsoniclabs/tosca/go/ct/st"
	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/0xsoniclabs/tosca/go/tosca/vm"
)

// TestCtAdapter_Eip8024RulesMatchGeth cross-checks the conformance test rules
// for DUPN, SWAPN and EXCHANGE against geth, the reference implementation of
// EIP-8024. Every operand is covered, including the ones the rules only reach
// through their generic conditions rather than through a test value.
//
// The adapter's revision check is bypassed because Amsterdam as a whole is
// still blocked, see newestSupportedRevision.
func TestCtAdapter_Eip8024RulesMatchGeth(t *testing.T) {
	for _, op := range []vm.OpCode{vm.DUPN, vm.SWAPN, vm.EXCHANGE} {
		for operand := 0; operand < 256; operand++ {
			for _, stackSize := range eip8024StackSizes(op, byte(operand)) {
				name := fmt.Sprintf("%v/0x%02x/%d", op, operand, stackSize)
				t.Run(name, func(t *testing.T) {
					input := eip8024State(op, byte(operand), stackSize)
					defer input.Release()

					rules := spc.Spec.GetRulesFor(input)
					if len(rules) == 0 {
						t.Fatalf("no rule covers %v with operand 0x%02x and %d stack elements", op, operand, stackSize)
					}

					want := input.Clone()
					defer want.Release()
					rules[0].Effect.Apply(want)

					got, err := stepN(input.Clone(), 1)
					if err != nil {
						t.Fatalf("failed to run geth: %v", err)
					}
					defer got.Release()

					// The rules do not model the return data of a step, and
					// geth does not report a program counter for halted runs.
					got.ReturnData = want.ReturnData
					if got.Status != st.Running {
						got.Pc = want.Pc
					}

					if !got.Eq(want) {
						t.Errorf("unexpected result, diff: %v", got.Diff(want))
					}
				})
			}
		}
	}
}

// eip8024StackSizes returns the stack sizes worth testing for the given
// operand: the ones just below, at, and just above the number of elements the
// instruction requires.
func eip8024StackSizes(op vm.OpCode, operand byte) []int {
	if !vm.IsValidImmediateOperand(op, operand) {
		return []int{0, 1, st.MaxStackSize}
	}
	required := vm.MinStackSizeForImmediate(op, operand)
	sizes := []int{required - 1, required, required + 1}
	if op == vm.DUPN {
		// DUPN pushes, so the upper end of the stack is a boundary as well.
		sizes = append(sizes, st.MaxStackSize-1, st.MaxStackSize)
	}
	return sizes
}

func eip8024State(op vm.OpCode, operand byte, stackSize int) *st.State {
	state := st.NewState(st.NewCode([]byte{byte(op), operand}))
	state.Status = st.Running
	state.Revision = tosca.R16_Amsterdam
	state.BlockContext.BlockNumber = common.GetForkBlock(tosca.R16_Amsterdam)
	state.Pc = 0
	state.Gas = 1000
	state.Stack = st.NewStackWithSize(stackSize)
	for i := 0; i < stackSize; i++ {
		// Distinct values, so that a swap of the wrong positions is visible.
		state.Stack.Set(i, common.NewU256(uint64(i)+1))
	}
	return state
}
