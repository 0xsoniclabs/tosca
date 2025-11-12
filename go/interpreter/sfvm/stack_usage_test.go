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
)

func TestComputeStackUsage_ProducesValidResultsForSingleOps(t *testing.T) {
	tests := []struct {
		op    OpCode
		usage stackUsage
	}{
		{STOP, stackUsage{from: 0, to: 0, delta: 0}},
		{ADD, stackUsage{from: -2, to: 0, delta: -1}},
		{POP, stackUsage{from: -1, to: 0, delta: -1}},
		{PUSH5, stackUsage{from: 0, to: 1, delta: 1}},
		{SWAP1, stackUsage{from: -2, to: 0, delta: 0}},
		{SWAP10, stackUsage{from: -11, to: 0, delta: 0}},
		{DUP1, stackUsage{from: -1, to: 1, delta: 1}},
		{DUP12, stackUsage{from: -12, to: 1, delta: 1}},
		{LOG3, stackUsage{from: -5, to: 0, delta: -5}},
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
