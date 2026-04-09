// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package gen

import (
	"errors"
	"testing"

	"pgregory.net/rand"
)

func TestSelfDestructedGenerator_UnconstrainedGeneratorCanGenerate(t *testing.T) {
	rnd := rand.New(0)
	generator := NewSelfDestructedGenerator()
	_, _, err := generator.Generate(rnd)
	if err != nil {
		t.Errorf("unexpected error during generation: %v", err)
	}
}

func TestSelfDestructedGenerator_SelfDestructedConstraintIsEnforced(t *testing.T) {
	rnd := rand.New(0)

	tests := map[string]struct {
		wantGenerated   bool
		constraintEffet func(g *SelfDestructedGenerator)
	}{
		"SelfDestruct":    {true, func(g *SelfDestructedGenerator) { g.MarkAsSelfDestructed() }},
		"NotSelfDestruct": {false, func(g *SelfDestructedGenerator) { g.MarkAsNotSelfDestructed() }},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			generator := NewSelfDestructedGenerator()
			test.constraintEffet(generator)
			hasSelfDestructed, _, err := generator.Generate(rnd)

			if err != nil {
				t.Errorf("Unexpected error during generation: %v", err)
			}

			if hasSelfDestructed != test.wantGenerated {
				t.Errorf("unexpected generates has-self-destructed value")
			}
		})
	}
}

func TestSelfDestructedGenerator_IsNewContractConstraintIsEnforced(t *testing.T) {
	rnd := rand.New(0)

	tests := map[string]struct {
		wantGenerated   bool
		constraintEffet func(g *SelfDestructedGenerator)
	}{
		"newContract":    {true, func(g *SelfDestructedGenerator) { g.MarkAsNewContract() }},
		"NotNewContract": {false, func(g *SelfDestructedGenerator) { g.MarkAsNotNewContract() }},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			generator := NewSelfDestructedGenerator()
			test.constraintEffet(generator)
			_, isNewContract, err := generator.Generate(rnd)

			if err != nil {
				t.Errorf("Unexpected error during generation: %v", err)
			}

			if isNewContract != test.wantGenerated {
				t.Errorf("unexpected is-new-contract value")
			}
		})
	}
}

func TestSelfDestructedGenerator_ConflictingConstraintsAreDetected(t *testing.T) {
	tests := map[string]func(g *SelfDestructedGenerator){
		"self-destructed constraints": func(g *SelfDestructedGenerator) {
			g.MarkAsSelfDestructed()
			g.MarkAsNotSelfDestructed()
		},
		"new-contract constraints": func(g *SelfDestructedGenerator) {
			g.MarkAsNewContract()
			g.MarkAsNotNewContract()
		},
	}

	for name, setup := range tests {
		t.Run(name, func(t *testing.T) {
			rnd := rand.New(0)
			generator := NewSelfDestructedGenerator()
			setup(generator)

			_, _, err := generator.Generate(rnd)
			if !errors.Is(err, ErrUnsatisfiable) {
				t.Errorf("Conflicting constraints not detected")
			}
		})
	}
}

func TestSelfDestructedGenerator_String(t *testing.T) {
	tests := map[string]struct {
		mustBeSelfDestructed    bool
		mustNotBeSelfDestructed bool
		mustBeNewContract       bool
		mustNotBeNewContract    bool
		want                    string
	}{
		"no constraints": {
			want: "{true}",
		},
		"conflict selfdestructed": {
			mustBeSelfDestructed:    true,
			mustNotBeSelfDestructed: true,
			want:                    "{false}",
		},
		"conflict new contract": {
			mustBeNewContract:    true,
			mustNotBeNewContract: true,
			want:                 "{false}",
		},
		"must be self-destructed": {
			mustBeSelfDestructed: true,
			want:                 "{mustBeSelfDestructed}",
		},
		"must not be self-destructed": {
			mustNotBeSelfDestructed: true,
			want:                    "{mustNotBeSelfDestructed}",
		},
		"must be new contract": {
			mustBeNewContract: true,
			want:              "{mustBeNewContract}",
		},
		"must not be new contract": {
			mustNotBeNewContract: true,
			want:                 "{mustNotBeNewContract}",
		},
		"must be self-destructed and new contract": {
			mustBeSelfDestructed: true,
			mustBeNewContract:    true,
			want:                 "{mustBeSelfDestructed, mustBeNewContract}",
		},
		"must not be self-destructed and not be new contract": {
			mustNotBeSelfDestructed: true,
			mustNotBeNewContract:    true,
			want:                    "{mustNotBeSelfDestructed, mustNotBeNewContract}",
		},
		"must be self-destructed and not be new contract": {
			mustBeSelfDestructed: true,
			mustNotBeNewContract: true,
			want:                 "{mustBeSelfDestructed, mustNotBeNewContract}",
		},
		"must not be self-destructed and be new contract": {
			mustNotBeSelfDestructed: true,
			mustBeNewContract:       true,
			want:                    "{mustNotBeSelfDestructed, mustBeNewContract}",
		},
	}
	for name, values := range tests {
		t.Run(name, func(t *testing.T) {
			generator := NewSelfDestructedGenerator()
			if values.mustBeSelfDestructed {
				generator.MarkAsSelfDestructed()
			}
			if values.mustNotBeSelfDestructed {
				generator.MarkAsNotSelfDestructed()
			}
			if values.mustBeNewContract {
				generator.MarkAsNewContract()
			}
			if values.mustNotBeNewContract {
				generator.MarkAsNotNewContract()
			}
			str := generator.String()
			if str != values.want {
				t.Errorf("unexpected string: wanted %v, but got %v", values.want, str)
			}
		})
	}
}

func TestSelfDestructedGenerator_Restore(t *testing.T) {
	gen1 := NewSelfDestructedGenerator()
	gen2 := NewSelfDestructedGenerator()
	gen2.mustNotBeSelfDestructed = true
	gen2.mustBeSelfDestructed = true
	gen2.mustNotBeNewContract = true
	gen2.mustBeNewContract = true

	gen1.Restore(gen2)
	if !gen1.mustNotBeSelfDestructed || !gen1.mustBeSelfDestructed ||
		!gen1.mustNotBeNewContract || !gen1.mustBeNewContract {
		t.Error("selfDestructedGenerator's restore is broken")
	}
}
