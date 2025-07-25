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
	"testing"

	"pgregory.net/rand"

	"github.com/0xsoniclabs/tosca/go/ct"
	. "github.com/0xsoniclabs/tosca/go/ct/common"
	"github.com/0xsoniclabs/tosca/go/ct/gen"
	. "github.com/0xsoniclabs/tosca/go/ct/rlz"
	"github.com/0xsoniclabs/tosca/go/ct/spc"
	"github.com/0xsoniclabs/tosca/go/ct/st"
	"github.com/0xsoniclabs/tosca/go/interpreter/evmzero"
	"github.com/0xsoniclabs/tosca/go/interpreter/geth"
	"github.com/0xsoniclabs/tosca/go/interpreter/lfvm"
	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/0xsoniclabs/tosca/go/tosca/vm"
)

var evms = map[string]ct.Evm{
	"geth":    geth.NewConformanceTestingTarget(),
	"lfvm":    lfvm.NewConformanceTestingTarget(),
	"evmzero": evmzero.NewConformanceTestingTarget(),
}

func TestCt_ExplicitCases(t *testing.T) {
	tests := map[string]Condition{}
	for _, revision := range AllSupportedRevisions() {
		tests["jump_to_2^32_"+revision.String()] =
			And(
				IsRevision(revision),
				Eq(Status(), st.Running),
				Eq(Op(Pc()), vm.JUMP),
				Eq(Op(Constant(NewU256(0))), vm.JUMPDEST),
				Eq(Param(0), NewU256(1<<32)),
				Ge(Gas(), tosca.Gas(8)),
			)
		tests["jumpi_to_2^32"+revision.String()] =
			And(
				IsRevision(revision),
				Eq(Status(), st.Running),
				Eq(Op(Pc()), vm.JUMPI),
				Eq(Op(Constant(NewU256(0))), vm.JUMPDEST),
				Eq(Param(0), NewU256(1<<32)),
				Ne(Param(1), NewU256(0)),
				Ge(Gas(), tosca.Gas(10)),
			)
	}

	random := rand.New(0)
	for name, condition := range tests {
		t.Run(name, func(t *testing.T) {
			generator := gen.NewStateGenerator()
			condition.Restrict(generator)

			input, err := generator.Generate(random)
			if err != nil {
				t.Fatalf("failed to generate satisfying state for %v: %v", condition, err)
			}
			if ok, err := condition.Check(input); !ok || err != nil {
				t.Fatalf("failed to generate satisfying state for %v, got %v, satisfying: %v, error:%v", condition, input, ok, err)
			}

			rules := spc.Spec.GetRulesFor(input)
			if len(rules) == 0 {
				t.Fatalf("no rule for test state: %v", input)
			}

			output := input.Clone()
			rules[0].Effect.Apply(output)

			for name, evm := range evms {
				t.Run(name, func(t *testing.T) {
					res, err := evm.StepN(input.Clone(), 1)
					if err != nil {
						t.Fatalf("failed to run test case: %v", err)
					}
					if !res.Eq(output) {
						t.Errorf("Invalid result, wanted %v, got %v", output, res)
						for _, diff := range output.Diff(res) {
							t.Error(diff)
						}
					}
				})
			}
		})
	}
}
