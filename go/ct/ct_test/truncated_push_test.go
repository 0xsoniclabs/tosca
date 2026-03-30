// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package ct_test

import (
	"bytes"
	"fmt"
	"testing"

	"github.com/0xsoniclabs/tosca/go/ct/common"
	"github.com/0xsoniclabs/tosca/go/ct/st"
	"github.com/0xsoniclabs/tosca/go/tosca/vm"
	"github.com/stretchr/testify/require"
)

func TestPushInstructions_PadDataWithZeroIfInsufficientDataIsProvided(t *testing.T) {
	for vmName, targetVm := range evms {
		for pushSize := 1; pushSize <= 32; pushSize++ {
			for _, dataSize := range []int{0, pushSize / 2, pushSize - 1, pushSize} {
				name := fmt.Sprintf("%s/Push%d/%d data elements", vmName, pushSize, dataSize)
				t.Run(name, func(t *testing.T) {
					code := []byte{
						byte(byte(vm.PUSH1) + byte(pushSize) - 1), // PUSHX
					}
					code = append(code, bytes.Repeat([]byte{byte(0xFF)}, dataSize)...)

					state := st.NewState(st.NewCode(code))
					state.Stack = st.NewStack()
					state.Gas = 3

					resultState, err := targetVm.StepN(state, 1)
					if err != nil {
						t.Fatalf("failed to run test case: %v", err)
					}

					expected := bytes.Repeat([]byte{0xFF}, dataSize)
					expected = append(expected, bytes.Repeat([]byte{0x00}, pushSize-dataSize)...)
					require.Equal(t, common.NewU256FromBytes(expected...),
						resultState.Stack.Pop(), "Unexpected pushed stack value")
				})
			}
		}
	}
}
