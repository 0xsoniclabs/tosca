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

	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/0xsoniclabs/tosca/go/tosca/vm"
)

func TestAnalysis_NewAnalysisIsNonEmpty(t *testing.T) {
	a := newAnalysis(10)
	if a.size == 0 {
		t.Error("expected newAnalysis to return a non-empty Analysis")
	}
	if len(a.data) == 0 {
		t.Error("expected newAnalysis to return a non-empty data slice")
	}
}

func TestAnalysis_MarkJumpDestAndIsJumpDest(t *testing.T) {
	size := 10
	a := newAnalysis(uint64(size))
	a.markJumpDest(2)
	a.markJumpDest(18)
	// Check that the jump destination is marked correctly over boundaries
	for i := 0; i < 2*size; i++ {
		if i == 2 && !a.isJumpDest(uint64(i)) {
			t.Errorf("expected index %d to be marked as jump destination", i)
		}
		if i != 2 && a.isJumpDest(uint64(i)) {
			t.Errorf("expected index %d to not be marked as jump destination", i)
		}
	}
}

func TestAnalysis_MarksJumpDestCorrectly(t *testing.T) {
	code := tosca.Code{byte(vm.JUMPDEST), byte(vm.PUSH1), byte(vm.JUMPDEST), byte(vm.JUMPDEST)}
	analysis := jumpDestAnalysisInternal(code)
	if !analysis.isJumpDest(0) {
		t.Errorf("expected index 0 to be jump destination")
	}
	if analysis.isJumpDest(1) {
		t.Errorf("expected index 1 to not be jump destination")
	}
	if analysis.isJumpDest(2) {
		t.Errorf("expected index 2 to not be jump destination")
	}
	if !analysis.isJumpDest(3) {
		t.Errorf("expected index 3 to be jump destination")
	}

}

func TestAnalysis_PushDataIsSkipped(t *testing.T) {
	code := tosca.Code{
		byte(vm.PUSH9), byte(vm.JUMPDEST), byte(vm.JUMPDEST), byte(vm.JUMPDEST), byte(vm.JUMPDEST),
		byte(vm.JUMPDEST), byte(vm.JUMPDEST), byte(vm.JUMPDEST), byte(vm.JUMPDEST), byte(vm.JUMPDEST),
		byte(vm.JUMPDEST),
		byte(vm.PUSH2), byte(vm.JUMPDEST), byte(vm.JUMPDEST),
		byte(vm.JUMPDEST),
	}
	analysis := jumpDestAnalysisInternal(code)
	for i := range code {
		if analysis.isJumpDest(uint64(i)) && (i != 10 && i != 14) {
			t.Errorf("expected index %d to be jump destination", i)
		}
		if !analysis.isJumpDest(uint64(i)) && (i == 10 || i == 14) {
			t.Errorf("expected index %d to not be jump destination", i)
		}
	}
}
