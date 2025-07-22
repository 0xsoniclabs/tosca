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
	"fmt"
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

func TestCombineStackUsage(t *testing.T) {
	tests := []struct {
		ops   []vm.OpCode
		usage stackUsage
	}{
		{
			[]vm.OpCode{},
			stackUsage{from: 0, to: 0, delta: 0},
		},
		{
			[]vm.OpCode{vm.PUSH1},
			stackUsage{from: 0, to: 1, delta: 1},
		},
		{
			[]vm.OpCode{vm.POP},
			stackUsage{from: -1, to: 0, delta: -1},
		},
		{
			[]vm.OpCode{vm.PUSH1, vm.PUSH1},
			stackUsage{from: 0, to: 2, delta: 2},
		},
		{
			[]vm.OpCode{vm.PUSH1, vm.POP},
			stackUsage{from: 0, to: 1, delta: 0},
		},
		{
			[]vm.OpCode{vm.POP, vm.PUSH1},
			stackUsage{from: -1, to: 0, delta: 0},
		},
		{
			[]vm.OpCode{vm.POP, vm.POP},
			stackUsage{from: -2, to: 0, delta: -2},
		},
		{
			[]vm.OpCode{vm.PUSH1, vm.PUSH1, vm.POP, vm.POP},
			stackUsage{from: 0, to: 2, delta: 0},
		},
		{
			[]vm.OpCode{vm.PUSH1, vm.PUSH1, vm.POP, vm.POP, vm.POP, vm.PUSH1, vm.PUSH1},
			stackUsage{from: -1, to: 2, delta: 1},
		},
		{
			[]vm.OpCode{vm.PUSH1, vm.LOG4, vm.PUSH1},
			stackUsage{from: -5, to: 1, delta: -4},
		},
		{
			[]vm.OpCode{vm.PUSH1, vm.ADD, vm.ISZERO, vm.PUSH2, vm.JUMPI},
			stackUsage{from: -1, to: 1, delta: -1},
		},
	}

	for _, test := range tests {
		t.Run(fmt.Sprintf("%v", test.ops), func(t *testing.T) {
			usages := []stackUsage{}
			for _, op := range test.ops {
				usages = append(usages, computeStackUsage(op))
			}

			res := combineStackUsage(usages...)
			if res != test.usage {
				t.Errorf("unexpected result: want %v, got %v", test.usage, res)
			}
		})
	}
}

// combineStackUsage combines the given stack usages into a single stack usage.
func combineStackUsage(usages ...stackUsage) stackUsage {
	// This function simulates the effect of the given stack usages on the stack
	// step by step. The delta of the resulting stack usage tracks the current
	// stack height offset.
	res := stackUsage{}
	for _, usage := range usages {
		from := usage.from + res.delta
		to := usage.to + res.delta

		if from < res.from {
			res.from = from
		}
		if to > res.to {
			res.to = to
		}
		res.delta += usage.delta
	}
	return res
}
