// Copyright (c) 2024 Fantom Foundation
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at fantom.foundation/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package gen

import (
	"errors"
	"maps"
	"strings"
	"testing"

	. "github.com/0xsoniclabs/tosca/go/ct/common"
	"github.com/0xsoniclabs/tosca/go/tosca"
	"pgregory.net/rand"
)

func TestAccountsGenerator_UnconstrainedGeneratorCanProduceBalance(t *testing.T) {
	rnd := rand.New(0)
	generator := NewAccountGenerator()
	accountAddress := RandomAddress(rnd)
	if _, err := generator.Generate(nil, rnd, accountAddress); err != nil {
		t.Errorf("Unexpected error during generation: %v", err)
	}
}

func TestAccountsGenerator_WarmConstraintIsEnforced(t *testing.T) {
	v1 := Variable("v1")
	assignment := Assignment{}
	assignment[v1] = NewU256(42)

	rnd := rand.New(0)
	generator := NewAccountGenerator()
	accountAddress := RandomAddress(rnd)
	generator.BindWarm(v1)
	accounts, err := generator.Generate(assignment, rnd, accountAddress)
	if err != nil {
		t.Errorf("Unexpected error during generation: %v", err)
	}

	if !accounts.IsWarm(NewAddressFromInt(42)) {
		t.Errorf("Expected constraint address to be warm, but generated state marked as cold")
	}
}

func TestAccountsGenerator_ConflictingWarmColdConstraintsAreDetected(t *testing.T) {
	v1 := Variable("v1")
	assignment := Assignment{}
	assignment[v1] = NewU256(42)

	rnd := rand.New(0)
	generator := NewAccountGenerator()
	accountAddress := NewAddress(NewU256(42))

	generator.BindCold(v1)
	generator.BindWarm(v1)

	_, err := generator.Generate(assignment, rnd, accountAddress)
	if !errors.Is(err, ErrUnsatisfiable) {
		t.Errorf("Conflicting warm/cold constraints not detected")
	}
}

func TestAccountsGenerator_WarmColdConstraintsNoAssignment(t *testing.T) {
	v1 := Variable("v1")
	v2 := Variable("v2")
	assignment := Assignment{}
	rnd := rand.New(0)
	generator := NewAccountGenerator()

	generator.BindWarm(v1)
	generator.BindCold(v2)

	accounts, err := generator.Generate(assignment, rnd, NewAddressFromInt(8))
	if err != nil {
		t.Fatalf("Unexpected error during account generation")
	}

	pos1, found1 := assignment[v1]
	pos2, found2 := assignment[v2]

	if !found1 || !found2 {
		t.Fatalf("Variable not bound by generator")
	}
	if !accounts.IsWarm(NewAddress(pos1)) {
		t.Errorf("Expected address to be warm but got cold")
	}
	if accounts.IsWarm(NewAddress(pos2)) {
		t.Errorf("Expected address to be cold but got warm")
	}
}

func TestAccountsGenerator_DelegationDesignatorCanBePrinted(t *testing.T) {

	tests := map[string]struct {
		setup    func(*AccountsGenerator, Variable)
		expected string
	}{
		"no delegation designator": {
			setup: func(gen *AccountsGenerator, v Variable) {
				gen.BindToAddressOfNotDelegatingAccount(v)
			},
			expected: "{!isDelegated($v1)}",
		},
		"delegate is cold": {
			setup: func(gen *AccountsGenerator, v Variable) {
				gen.BindToAddressOfDelegatingAccount(v, tosca.ColdAccess)
			},
			expected: "{isDelegated($v1),cold(delegateOf($v1))}",
		},
		"delegate is warm": {
			setup: func(gen *AccountsGenerator, v Variable) {
				gen.BindToAddressOfDelegatingAccount(v, tosca.WarmAccess)
			},
			expected: "{isDelegated($v1),warm(delegateOf($v1))}",
		},
		"is sorted": {
			setup: func(gen *AccountsGenerator, v Variable) {
				v2 := Variable("v2")
				gen.BindToAddressOfDelegatingAccount(v2, tosca.WarmAccess)
				gen.BindToAddressOfNotDelegatingAccount(v)
			},
			expected: "{!isDelegated($v1),isDelegated($v2),warm(delegateOf($v2))}",
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {

			v1 := Variable("v1")

			generator := NewAccountGenerator()
			test.setup(generator, v1)

			print := generator.String()
			if print != test.expected {
				t.Errorf("Expected %v but got %v", test.expected, print)
			}
		})
	}
}

func TestAccountsGenerator_DelegationDesignatorConstraintsAreUsed(t *testing.T) {

	tests := map[string]struct {
		setup        func(*AccountsGenerator, Variable)
		withDelegate bool
		access       tosca.AccessStatus
	}{
		"no delegation designator": {
			setup: func(gen *AccountsGenerator, v Variable) {
				gen.BindToAddressOfNotDelegatingAccount(v)
			},
			withDelegate: false,
		},
		"delegate is cold": {
			setup: func(gen *AccountsGenerator, v Variable) {
				gen.BindToAddressOfDelegatingAccount(v, tosca.ColdAccess)
			},
			withDelegate: true,
			access:       tosca.ColdAccess,
		},
		"delegate is warm": {
			setup: func(gen *AccountsGenerator, v Variable) {
				gen.BindToAddressOfDelegatingAccount(v, tosca.WarmAccess)
			},
			withDelegate: true,
			access:       tosca.WarmAccess,
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			rand := rand.New(0)

			v1 := Variable("v1")

			generator := NewAccountGenerator()
			test.setup(generator, v1)

			assignment := Assignment{}
			accounts, err := generator.Generate(assignment, rand, NewAddressFromInt(8))
			if err != nil {
				t.Fatalf("Unexpected error during account generation")
			}

			variableAssignment, assigned := assignment[v1]
			if !assigned {
				t.Fatalf("Variable not bound by generator")
			}
			calleeAddress := NewAddress(variableAssignment)

			if !test.withDelegate {
				if accounts.Exists(calleeAddress) {
					account := accounts.GetAccount(calleeAddress)
					_, isDesignator := ParseDelegationDesignator(account.Code)
					if isDesignator {
						t.Errorf("Expected address not to be a delegation designator but it is")
					}
				}
			} else {
				if !accounts.Exists(calleeAddress) {
					t.Errorf("Expected address to be a delegation designator but it is not")
				}
				account := accounts.GetAccount(calleeAddress)
				delegateAddress, isDesignator := ParseDelegationDesignator(account.Code)
				if !isDesignator {
					t.Errorf("Expected address to be a delegation designator but it is not")
				} else {
					if test.access == tosca.WarmAccess && !accounts.IsWarm(delegateAddress) {
						t.Errorf("Expected address to be warm but it is not")
					}
					if test.access == tosca.ColdAccess && accounts.IsWarm(delegateAddress) {
						t.Errorf("Expected address to be warm but it is not")
					}
				}
			}
		})
	}
}

func TestAccountsGenerator_DelegationDesignatorCanConflictWithOtherRules(t *testing.T) {
	test := map[string]struct {
		constraint func(*AccountsGenerator, Variable)
	}{
		"disable and warm": {
			constraint: func(gen *AccountsGenerator, v Variable) {
				gen.BindToAddressOfNotDelegatingAccount(v)
				gen.BindToAddressOfDelegatingAccount(v, tosca.WarmAccess)
			},
		},
		"disable and cold": {
			constraint: func(gen *AccountsGenerator, v Variable) {
				gen.BindToAddressOfNotDelegatingAccount(v)
				gen.BindToAddressOfDelegatingAccount(v, tosca.ColdAccess)
			},
		},
		"cold and warm": {
			constraint: func(gen *AccountsGenerator, v Variable) {
				gen.BindToAddressOfDelegatingAccount(v, tosca.ColdAccess)
				gen.BindToAddressOfDelegatingAccount(v, tosca.WarmAccess)
			},
		},
		"empty and contains designator conflict": {
			constraint: func(gen *AccountsGenerator, v Variable) {
				gen.BindToAddressOfDelegatingAccount(v, tosca.ColdAccess)
				gen.BindToAddressOfEmptyAccount(v)
			},
		},
	}

	for name, test := range test {
		t.Run(name, func(t *testing.T) {
			rand := rand.New(0)

			v1 := Variable("v1")

			generator := NewAccountGenerator()
			test.constraint(generator, v1)

			assignment := Assignment{}
			_, err := generator.Generate(assignment, rand, NewAddressFromInt(8))
			if !errors.Is(err, ErrUnsatisfiable) {
				t.Errorf("Conflicting constraints not detected")
			}
		})
	}
}

func TestAccountsGenerator_CanSpecifyEmptyConstraints(t *testing.T) {
	v1 := Variable("v1")
	v2 := Variable("v2")
	v3 := Variable("v3")

	gen := NewAccountGenerator()

	gen.BindToAddressOfEmptyAccount(v1)
	gen.BindToAddressOfNonEmptyAccount(v2)
	gen.BindToAddressOfEmptyAccount(v3)
	gen.BindToAddressOfNonEmptyAccount(v3)

	print := gen.String()

	want := []string{
		"empty($v1)",
		"!empty($v2)",
		"empty($v3)",
		"!empty($v3)",
	}

	for _, w := range want {
		if !strings.Contains(print, w) {
			t.Errorf("Expected to find %q in %q", w, print)
		}
	}
}

func TestAccountsGenerator_EmptinessConstraintsAreSatisfied(t *testing.T) {
	v1 := Variable("v1")
	v2 := Variable("v2")
	assignment := Assignment{}
	rnd := rand.New()
	generator := NewAccountGenerator()

	generator.BindToAddressOfEmptyAccount(v1)
	generator.BindToAddressOfNonEmptyAccount(v2)

	accounts, err := generator.Generate(assignment, rnd, NewAddressFromInt(8))
	if err != nil {
		t.Fatalf("Unexpected error during account generation")
	}

	addr1 := NewAddress(assignment[v1])
	addr2 := NewAddress(assignment[v2])

	if !accounts.IsEmpty(addr1) {
		t.Errorf("Expected account to be empty but it does not")
	}
	if accounts.IsEmpty(addr2) {
		t.Errorf("Expected account not to empty but it does")
	}
}

func TestAccountsGenerator_PreAssignedVariablesArePreserved(t *testing.T) {
	v1 := Variable("v1")
	v2 := Variable("v2")

	assignment := Assignment{
		v1: NewU256(42),
		v2: NewU256(24),
	}
	backup := maps.Clone(assignment)

	rnd := rand.New()
	generator := NewAccountGenerator()

	generator.BindToAddressOfEmptyAccount(v1)

	accounts, err := generator.Generate(assignment, rnd, tosca.Address{})
	if err != nil {
		t.Fatalf("Unexpected error during account generation")
	}

	addr1 := NewAddress(assignment[v1])
	if !accounts.IsEmpty(addr1) {
		t.Errorf("Expected account to be empty but it does not")
	}

	if !maps.Equal(backup, assignment) {
		t.Errorf("Pre-assigned variables were modified")
	}
}

func TestAccountsGenerator_ConflictingEmptinessConstraintsAreDetected(t *testing.T) {
	v1 := Variable("v1")
	assignment := Assignment{}

	rnd := rand.New()
	generator := NewAccountGenerator()

	generator.BindToAddressOfEmptyAccount(v1)
	generator.BindToAddressOfNonEmptyAccount(v1)

	_, err := generator.Generate(assignment, rnd, tosca.Address{})
	if !errors.Is(err, ErrUnsatisfiable) {
		t.Errorf("Conflicting emptiness constraints not detected")
	}
}

func TestAccountsGenerator_ConflictingBetweenPreAssignmentAndEmptinessConstraintsIsDetected(t *testing.T) {
	v1 := Variable("v1")
	v2 := Variable("v2")

	rnd := rand.New()
	generator := NewAccountGenerator()

	generator.BindToAddressOfEmptyAccount(v1)
	generator.BindToAddressOfNonEmptyAccount(v2)

	assignment := Assignment{}
	assignment[v1] = NewU256(42)
	assignment[v2] = NewU256(42)

	_, err := generator.Generate(assignment, rnd, tosca.Address{})
	if !errors.Is(err, ErrUnsatisfiable) {
		t.Errorf("Conflicting emptiness constraints not detected")
	}
}

func TestAccountsGenerator_CanHandleAccessStateAndEmptinessConstraintsOnSameVariable(t *testing.T) {
	v1 := Variable("v1")
	assignment := Assignment{}

	rnd := rand.New()
	generator := NewAccountGenerator()

	generator.BindToAddressOfEmptyAccount(v1)
	generator.BindWarm(v1)

	accounts, err := generator.Generate(assignment, rnd, tosca.Address{})
	if err != nil {
		t.Fatalf("Unexpected error during account generation")
	}

	address := NewAddress(assignment[v1])
	if !accounts.IsEmpty(address) {
		t.Errorf("Expected account to be empty but it does not")
	}

	if !accounts.IsWarm(address) {
		t.Errorf("Expected account to be warm but it is cold")
	}
}

func TestAccountsGenerator_CanSpecifyBalanceConstraints(t *testing.T) {
	v1 := Variable("v1")
	v2 := Variable("v2")

	gen := NewAccountGenerator()

	gen.AddBalanceLowerBound(v1, NewU256(42))
	gen.AddBalanceUpperBound(v1, NewU256(76))
	gen.AddBalanceUpperBound(v2, NewU256(52))

	print := gen.String()

	want := []string{
		"balance($v1) ≥ 42",
		"balance($v1) ≤ 76",
		"balance($v2) ≤ 52",
	}

	for _, w := range want {
		if !strings.Contains(print, w) {
			t.Errorf("Expected to find %v in %v", w, print)
		}
	}
}

func TestAccountsGenerator_BalanceConstraintsAreEnforced(t *testing.T) {
	v1 := Variable("v1")
	v2 := Variable("v2")
	v3 := Variable("v3")

	gen := NewAccountGenerator()

	gen.AddBalanceLowerBound(v1, NewU256(42))
	gen.AddBalanceUpperBound(v1, NewU256(76))
	gen.AddBalanceUpperBound(v2, NewU256(52))
	gen.AddBalanceLowerBound(v3, NewU256(123))
	gen.AddBalanceUpperBound(v3, NewU256(123))

	assignment := Assignment{}
	rnd := rand.New(0)
	state, err := gen.Generate(assignment, rnd, tosca.Address{})
	if err != nil {
		t.Fatalf("Unexpected error during account generation")
	}

	addr1 := NewAddress(assignment[v1])
	addr2 := NewAddress(assignment[v2])
	addr3 := NewAddress(assignment[v3])

	if got := state.GetBalance(addr1); got.Lt(NewU256(42)) {
		t.Errorf("Expected balance to be at least 42 but got %v", got.DecimalString())
	}
	if got := state.GetBalance(addr1); got.Gt(NewU256(76)) {
		t.Errorf("Expected balance to be at most 76 but got %v", got.DecimalString())
	}
	if got := state.GetBalance(addr2); got.Gt(NewU256(52)) {
		t.Errorf("Expected balance to be at most 52 but got %v", got.DecimalString())
	}
	if got := state.GetBalance(addr3); got.Ne(NewU256(123)) {
		t.Errorf("Expected balance to be exactly 123 but got %v", got.DecimalString())
	}
}

func TestAccountsGenerator_BalanceConflictsAreDetected(t *testing.T) {
	v1 := Variable("v1")

	gen := NewAccountGenerator()

	gen.AddBalanceLowerBound(v1, NewU256(76))
	gen.AddBalanceUpperBound(v1, NewU256(24))

	assignment := Assignment{}
	rnd := rand.New(0)
	_, err := gen.Generate(assignment, rnd, tosca.Address{})
	if !errors.Is(err, ErrUnsatisfiable) {
		t.Errorf("Conflicting balance constraints not detected")
	}
}

func TestAccountsGenerator_PreAssignedAddressesAreRespected(t *testing.T) {
	v1 := Variable("v1")

	gen := NewAccountGenerator()

	gen.AddBalanceLowerBound(v1, NewU256(25))
	gen.AddBalanceUpperBound(v1, NewU256(25))

	assignment := Assignment{}
	assignment[v1] = NewU256(12345)
	rnd := rand.New(0)
	state, err := gen.Generate(assignment, rnd, tosca.Address{})
	if err != nil {
		t.Fatalf("Unexpected error during account generation")
	}

	if got := assignment[v1]; got.Ne(NewU256(12345)) {
		t.Errorf("Expected pre-assigned address to be preserved but got %v", got.DecimalString())
	}

	addr := NewAddress(assignment[v1])
	if got := state.GetBalance(addr); got.Ne(NewU256(25)) {
		t.Errorf("Expected balance to be exactly 25 but got %v", got.DecimalString())
	}
}

func TestAccountsGenerator_DelegationDesignatorConstraint_CanBeSorted(t *testing.T) {

	testValues := []*delegationDesignatorConstraint{
		{
			address:               Variable("v1"),
			isDelegated:           false,
			delegateAccountStatus: tosca.ColdAccess,
		},
		{
			address:               Variable("v2"),
			isDelegated:           false,
			delegateAccountStatus: tosca.ColdAccess,
		},
		{
			address:               Variable("v1"),
			isDelegated:           true,
			delegateAccountStatus: tosca.ColdAccess,
		},
		{
			address:               Variable("v2"),
			isDelegated:           true,
			delegateAccountStatus: tosca.ColdAccess,
		},
		{
			address:               Variable("v1"),
			isDelegated:           false,
			delegateAccountStatus: tosca.WarmAccess,
		},
		{
			address:               Variable("v2"),
			isDelegated:           false,
			delegateAccountStatus: tosca.WarmAccess,
		},
		{
			address:               Variable("v1"),
			isDelegated:           true,
			delegateAccountStatus: tosca.WarmAccess,
		},
		{
			address:               Variable("v2"),
			isDelegated:           true,
			delegateAccountStatus: tosca.WarmAccess,
		},
	}

	for _, a := range testValues {
		for _, b := range testValues {
			if a.Less(b) && b.Less(a) {
				t.Errorf("invalid comparison: a < b && b < a, a: %v, b %v", a, b)
			}
			if !a.Less(b) && !b.Less(a) && a != b {
				t.Errorf("invalid comparison: a != b but a!<b && b!<a, a: %v, b %v", a, b)
			}
		}
	}
}

func BenchmarkAccountGenWithConstraint(b *testing.B) {
	v1 := Variable("v1")
	v2 := Variable("v2")
	assignment := Assignment{}
	rnd := rand.New(0)
	generator := NewAccountGenerator()

	generator.BindWarm(v1)
	generator.BindCold(v2)
	for i := 0; i < b.N; i++ {
		_, err := generator.Generate(assignment, rnd, NewAddressFromInt(8))
		if err != nil {
			b.Fatalf("Invalid benchmark, Unexpected error during balance generation %v", err)
		}
	}
}

func BenchmarkAccountGenWithOutConstraint(b *testing.B) {
	assignment := Assignment{}
	rnd := rand.New(0)
	generator := NewAccountGenerator()

	for i := 0; i < b.N; i++ {
		_, err := generator.Generate(assignment, rnd, NewAddressFromInt(8))
		if err != nil {
			b.Fatalf("Invalid benchmark, Unexpected error during balance generation %v", err)
		}
	}
}
