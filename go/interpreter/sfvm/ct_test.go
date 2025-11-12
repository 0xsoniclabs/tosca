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
	"bytes"
	"errors"
	"testing"

	"github.com/0xsoniclabs/tosca/go/ct"
	cc "github.com/0xsoniclabs/tosca/go/ct/common"
	"github.com/0xsoniclabs/tosca/go/ct/st"
	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/0xsoniclabs/tosca/go/tosca/vm"
)

func TestCtAdapter_Add(t *testing.T) {
	s := st.NewState(st.NewCode([]byte{
		byte(vm.PUSH1), 3,
		byte(vm.PUSH1), 4,
		byte(vm.ADD),
	}))
	s.Status = st.Running
	s.Revision = tosca.R07_Istanbul
	s.Pc = 0
	s.Gas = 100
	s.Stack = st.NewStack(cc.NewU256(1), cc.NewU256(2))
	defer s.Stack.Release()
	s.Memory = st.NewMemory(1, 2, 3)

	c := NewConformanceTestingTarget()

	s, err := c.StepN(s, 4)

	if err != nil {
		t.Fatalf("unexpected conversion error: %v", err)
	}

	if want, got := st.Stopped, s.Status; want != got {
		t.Fatalf("unexpected status: wanted %v, got %v", want, got)
	}

	if want, got := cc.NewU256(3+4), s.Stack.Get(0); !want.Eq(got) {
		t.Errorf("unexpected result: wanted %s, got %s", want, got)
	}
}

func TestCtAdapter_Interface(t *testing.T) {
	// Compile time check that ctAdapter implements the st.Evm interface.
	var _ ct.Evm = &ctAdapter{}
}

func TestCtAdapter_ReturnsErrorForUnsupportedRevisions(t *testing.T) {
	unsupportedRevision := newestSupportedRevision + 1
	want := &tosca.ErrUnsupportedRevision{Revision: unsupportedRevision}
	s := st.NewState(st.NewCode([]byte{
		byte(vm.STOP),
	}))
	s.Revision = unsupportedRevision

	c := NewConformanceTestingTarget()
	_, err := c.StepN(s, 1)

	var e *tosca.ErrUnsupportedRevision
	if !errors.As(err, &e) {
		t.Errorf("unexpected error, wanted %v, got %v", want, err)
	}
}

func TestCtAdapter_DoesNotAffectNonRunningStates(t *testing.T) {
	s := st.NewState(st.NewCode([]byte{
		byte(vm.STOP),
	}))
	s.Status = st.Stopped

	c := NewConformanceTestingTarget()
	s2, err := c.StepN(s.Clone(), 1)
	if err != nil {
		t.Fatalf("unexpected conversion error: %v", err)
	}
	if !s.Eq(s2) {
		t.Errorf("unexpected state, wanted %v, got %v", s, s2)
	}
}

func TestCtAdapter_SetsPcOnResultingState(t *testing.T) {
	s := st.NewState(st.NewCode([]byte{
		byte(vm.PUSH1),
		0x01,
		byte(vm.PUSH0),
	}))
	s.Gas = 100
	s.Stack = st.NewStack()
	defer s.Stack.Release()
	c := NewConformanceTestingTarget()
	s2, err := c.StepN(s, 1)
	if err != nil {
		t.Fatalf("unexpected conversion error: %v", err)
	}
	if want, got := uint16(2), s2.Pc; want != got {
		t.Errorf("unexpected pc, wanted %d, got %d", want, got)
	}
}

func TestCtAdapter_FillsReturnDataOnResultingState(t *testing.T) {
	s := st.NewState(st.NewCode([]byte{
		byte(vm.PUSH1), byte(1),
		byte(vm.PUSH1), byte(0),
		byte(vm.RETURN),
	}))
	s.Gas = 100
	memory := []byte{0xFA}
	s.Memory.Append(memory)
	c := NewConformanceTestingTarget()
	s2, err := c.StepN(s, 3)
	if err != nil {
		t.Fatalf("unexpected conversion error: %v", err)
	}
	if want, got := memory, s2.ReturnData.ToBytes(); !bytes.Equal(want, got) {
		t.Errorf("unexpected return data, wanted %v, got %v", want, got)
	}
}

////////////////////////////////////////////////////////////
// ct -> sfvm

func TestConvertToSfvm_StatusCode(t *testing.T) {

	tests := map[status]st.StatusCode{
		statusRunning:        st.Running,
		statusReverted:       st.Reverted,
		statusReturned:       st.Stopped,
		statusStopped:        st.Stopped,
		statusSelfDestructed: st.Stopped,
		statusFailed:         st.Failed,
	}

	for status, test := range tests {
		got := convertSfvmStatusToCtStatus(status)
		if want, got := test, got; want != got {
			t.Errorf("unexpected conversion, wanted %v, got %v", want, got)
		}
	}
}

func TestConvertToSfvm_StatusCodeFailsOnUnknownStatus(t *testing.T) {
	status := convertSfvmStatusToCtStatus(statusFailed + 1)
	if status != st.Failed {
		t.Errorf("unexpected conversion, wanted %v, got %v", st.Failed, status)
	}
}

func TestConvertToSfvm_Stack(t *testing.T) {
	newSfvmStack := func(values ...cc.U256) *stack {
		stack := NewStack()
		for i := 0; i < len(values); i++ {
			value := values[i].Uint256()
			stack.push(&value)
		}
		return stack
	}

	tests := map[string]struct {
		ctStack   *st.Stack
		sfvmStack *stack
	}{
		"empty": {
			st.NewStack(),
			newSfvmStack()},
		"one-element": {
			st.NewStack(cc.NewU256(7)),
			newSfvmStack(cc.NewU256(7))},
		"two-elements": {
			st.NewStack(cc.NewU256(1), cc.NewU256(2)),
			newSfvmStack(cc.NewU256(1), cc.NewU256(2))},
		"three-elements": {
			st.NewStack(cc.NewU256(1), cc.NewU256(2), cc.NewU256(3)),
			newSfvmStack(cc.NewU256(1), cc.NewU256(2), cc.NewU256(3))},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			stack := convertCtStackToSfvmStack(test.ctStack)
			if want, got := test.sfvmStack.len(), stack.len(); want != got {
				t.Fatalf("unexpected stack size, wanted %v, got %v", want, got)
			}
			for i := 0; i < stack.len(); i++ {
				want := *test.sfvmStack.get(i)
				got := *stack.get(i)
				if want != got {
					t.Errorf("unexpected stack value, wanted %v, got %v", want, got)
				}
			}
			ReturnStack(test.sfvmStack)
			ReturnStack(stack)
			test.ctStack.Release()
		})
	}
}

////////////////////////////////////////////////////////////
// sfvm -> ct

func TestConvertToCt_Stack(t *testing.T) {
	newSfvmStack := func(values ...cc.U256) *stack {
		stack := NewStack()
		for i := 0; i < len(values); i++ {
			value := values[i].Uint256()
			stack.push(&value)
		}
		return stack
	}

	tests := map[string]struct {
		sfvmStack *stack
		ctStack   *st.Stack
	}{
		"empty": {
			newSfvmStack(),
			st.NewStack()},
		"one-element": {
			newSfvmStack(cc.NewU256(7)),
			st.NewStack(cc.NewU256(7))},
		"two-elements": {
			newSfvmStack(cc.NewU256(1), cc.NewU256(2)),
			st.NewStack(cc.NewU256(1), cc.NewU256(2))},
		"three-elements": {
			newSfvmStack(cc.NewU256(1), cc.NewU256(2), cc.NewU256(3)),
			st.NewStack(cc.NewU256(1), cc.NewU256(2), cc.NewU256(3))},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			want := test.ctStack
			ctStack := st.NewStack()
			got := convertSfvmStackToCtStack(test.sfvmStack, ctStack)

			diffs := got.Diff(want)
			for _, diff := range diffs {
				t.Errorf("%s", diff)
			}
			ReturnStack(test.sfvmStack)
			test.ctStack.Release()
			ctStack.Release()
		})
	}
}

func BenchmarkSfvmStackToCtStack(b *testing.B) {
	stack := NewStack()
	for i := 0; i < MAX_STACK_SIZE/2; i++ {
		stack.pushUndefined().SetUint64(uint64(i))
	}
	ctStack := st.NewStack()
	for i := 0; i < b.N; i++ {
		convertSfvmStackToCtStack(stack, ctStack)
	}
}
