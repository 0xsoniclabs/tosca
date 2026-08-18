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
	"math"
	"testing"

	"github.com/0xsoniclabs/tosca/go/ct/common"
	"github.com/0xsoniclabs/tosca/go/ct/spc"
	"github.com/0xsoniclabs/tosca/go/ct/st"
	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/0xsoniclabs/tosca/go/tosca/vm"
)

// TestCtAdapter_Eip7843RulesMatchGeth cross-checks the conformance test rules
// for SLOTNUM against geth, the reference implementation of EIP-7843.
//
// The adapter's revision check is bypassed because Amsterdam as a whole is
// still blocked, see newestSupportedRevision.
func TestCtAdapter_Eip7843RulesMatchGeth(t *testing.T) {
	for _, slotNumber := range []uint64{0, 1, 42, math.MaxUint64} {
		for _, gas := range []tosca.Gas{1, 2, 1000} {
			for _, stackSize := range []int{0, 1, st.MaxStackSize - 1, st.MaxStackSize} {
				name := fmt.Sprintf("%d/%d/%d", slotNumber, gas, stackSize)
				t.Run(name, func(t *testing.T) {
					input := eip7843State(slotNumber, gas, stackSize)
					defer input.Release()

					rules := spc.Spec.GetRulesFor(input)
					if len(rules) == 0 {
						t.Fatalf("no rule covers SLOTNUM with %d gas and %d stack elements", gas, stackSize)
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

func eip7843State(slotNumber uint64, gas tosca.Gas, stackSize int) *st.State {
	state := st.NewState(st.NewCode([]byte{byte(vm.SLOTNUM)}))
	state.Status = st.Running
	state.Revision = tosca.R16_Amsterdam
	state.BlockContext.BlockNumber = common.GetForkBlock(tosca.R16_Amsterdam)
	state.BlockContext.SlotNumber = slotNumber
	state.Pc = 0
	state.Gas = gas
	state.Stack = st.NewStackWithSize(stackSize)
	return state
}
