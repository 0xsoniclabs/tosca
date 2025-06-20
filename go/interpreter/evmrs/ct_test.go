// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package evmrs

import (
	"testing"

	"github.com/0xsoniclabs/tosca/go/ct"
	cc "github.com/0xsoniclabs/tosca/go/ct/common"
	"github.com/0xsoniclabs/tosca/go/ct/st"
	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/0xsoniclabs/tosca/go/tosca/vm"
)

func TestEvmrsCtAdapter_Add(t *testing.T) {
	s := st.NewState(st.NewCode([]byte{
		byte(vm.PUSH1), 3,
		byte(vm.PUSH1), 4,
		byte(vm.ADD),
		byte(vm.STOP),
	}))
	s.Status = st.Running
	s.Revision = tosca.R07_Istanbul
	s.Pc = 0
	s.Gas = 100
	s.Stack = st.NewStack()

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

func TestEvmrsCtAdapter_Interface(t *testing.T) {
	// Compile time check that ctAdapter implements the ct.Evm interface.
	var _ ct.Evm = ctAdapter{}
}
