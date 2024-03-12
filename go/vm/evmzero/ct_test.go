package evmzero

import (
	"testing"

	"github.com/Fantom-foundation/Tosca/go/ct"
	cc "github.com/Fantom-foundation/Tosca/go/ct/common"
	"github.com/Fantom-foundation/Tosca/go/ct/st"
)

func TestCtAdapter_Add(t *testing.T) {
	s := st.NewState(st.NewCode([]byte{
		byte(cc.PUSH1), 3,
		byte(cc.PUSH1), 4,
		byte(cc.ADD),
	}))
	s.Status = st.Running
	s.Revision = cc.R07_Istanbul
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

func TestCtAdapter_Interface(t *testing.T) {
	// Compile time check that ctAdapter implements the st.Evm interface.
	var _ ct.Evm = ctAdapter{}
}
