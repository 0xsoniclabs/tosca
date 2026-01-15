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
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca/vm"
)

func TestComputeStackUsage_ProducesValidResultsForSingleOps(t *testing.T) {
	tests := []struct {
		op    vm.OpCode
		usage stackUsage
	}{
		{vm.STOP, stackUsage{from: 0, to: 0, delta: 0}},
		{vm.ADD, stackUsage{from: -2, to: 0, delta: -1}},
		{vm.POP, stackUsage{from: -1, to: 0, delta: -1}},
		{vm.PUSH5, stackUsage{from: 0, to: 1, delta: 1}},
		{vm.SWAP1, stackUsage{from: -2, to: 0, delta: 0}},
		{vm.SWAP10, stackUsage{from: -11, to: 0, delta: 0}},
		{vm.DUP1, stackUsage{from: -1, to: 1, delta: 1}},
		{vm.DUP12, stackUsage{from: -12, to: 1, delta: 1}},
		{vm.LOG3, stackUsage{from: -5, to: 0, delta: -5}},
	}

	for _, test := range tests {
		t.Run(test.op.String(), func(t *testing.T) {
			usage := computeStackUsage(test.op)
			if got, want := usage, test.usage; got != want {
				t.Errorf("unexpected result: want %v, got %v", want, got)
			}
		})
	}
}
