// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package smt

import (
	"os/exec"
	"slices"
	"strings"
	"testing"

	"github.com/0xsoniclabs/tosca/go/ct/common"
	. "github.com/0xsoniclabs/tosca/go/ct/rlz"
	"github.com/0xsoniclabs/tosca/go/ct/spc"
	"github.com/0xsoniclabs/tosca/go/ct/st"
	"github.com/stretchr/testify/require"
)

func requireZ3(t *testing.T) {
	if _, err := exec.LookPath("z3"); err != nil {
		t.Skip("z3 is not installed")
	}
}

func rule(name string, effect Effect, conditions ...Condition) Rule {
	return Rule{Name: name, Condition: And(conditions...), Effect: effect}
}

func someEffect() Effect {
	return Change(func(*st.State) {})
}

func TestCheckCompleteness_AcceptsCoveringRules(t *testing.T) {
	requireZ3(t)
	model, err := CheckCompleteness([]Rule{
		rule("running", NoEffect(), Eq(Status(), st.Running)),
		rule("not_running", NoEffect(), Ne(Status(), st.Running)),
	})
	require.NoError(t, err)
	require.Empty(t, model)
}

func TestCheckCompleteness_ReportsUncoveredState(t *testing.T) {
	requireZ3(t)
	model, err := CheckCompleteness([]Rule{
		rule("running", NoEffect(), Eq(Status(), st.Running)),
	})
	require.NoError(t, err)
	require.Contains(t, model, "define-fun status")
}

func TestCheckCompleteness_UndefinedParametersDoNotMatch(t *testing.T) {
	requireZ3(t)
	model, err := CheckCompleteness([]Rule{
		rule("zero", NoEffect(), Eq(Param(0), common.NewU256(0))),
		rule("non_zero", NoEffect(), Ne(Param(0), common.NewU256(0))),
	})
	require.NoError(t, err)
	require.Contains(t, model, "define-fun stackSize () Int\n    0")
}

func TestCheckSoundness_AcceptsDisjointRules(t *testing.T) {
	requireZ3(t)
	conflicts, err := CheckSoundness([]Rule{
		rule("running", someEffect(), Eq(Status(), st.Running)),
		rule("not_running", someEffect(), Ne(Status(), st.Running)),
	}, 10)
	require.NoError(t, err)
	require.Empty(t, conflicts)
}

func TestCheckSoundness_ReportsOverlappingRules(t *testing.T) {
	requireZ3(t)
	conflicts, err := CheckSoundness([]Rule{
		rule("running", someEffect(), Eq(Status(), st.Running)),
		rule("low_gas", someEffect(), Lt(Gas(), 10)),
		rule("high_gas", someEffect(), Ge(Gas(), 10)),
	}, 10)
	require.NoError(t, err)
	require.Len(t, conflicts, 2)
	names := []string{}
	for _, conflict := range conflicts {
		require.Equal(t, "running", conflict.First.Name)
		names = append(names, conflict.Second.Name)
	}
	slices.Sort(names)
	require.Equal(t, []string{"high_gas", "low_gas"}, names)
}

func TestCheckSoundness_IgnoresOverlapsOfEquivalentEffects(t *testing.T) {
	requireZ3(t)
	conflicts, err := CheckSoundness([]Rule{
		rule("fails", FailEffect(), Eq(Status(), st.Running)),
		rule("fails_too", FailEffect(), Eq(Status(), st.Running), Lt(Gas(), 10)),
		rule("nothing", NoEffect(), Eq(Status(), st.Stopped)),
		rule("nothing_too", NoEffect(), Eq(Status(), st.Stopped), Lt(Gas(), 10)),
		rule("duplicate", someEffect(), Eq(Status(), st.Reverted)),
		rule("duplicate", someEffect(), Eq(Status(), st.Reverted)),
	}, 10)
	require.NoError(t, err)
	require.Empty(t, conflicts)
}

func TestCheckSoundness_DistinguishesRulesNamedLikeSpecialEffects(t *testing.T) {
	requireZ3(t)
	conflicts, err := CheckSoundness([]Rule{
		rule("none", someEffect(), Eq(Status(), st.Running)),
		rule("nothing", NoEffect(), Eq(Status(), st.Running)),
		rule("fail", someEffect(), Eq(Status(), st.Stopped)),
		rule("fails", FailEffect(), Eq(Status(), st.Stopped)),
	}, 10)
	require.NoError(t, err)
	pairs := []string{}
	for _, conflict := range conflicts {
		pairs = append(pairs, conflict.First.Name+"/"+conflict.Second.Name)
	}
	slices.Sort(pairs)
	require.Equal(t, []string{"fail/fails", "none/nothing"}, pairs)
}

func getSpecificationRules() []Rule {
	rules := spc.Spec.GetRules()
	slices.SortStableFunc(rules, func(a, b Rule) int {
		return strings.Compare(a.Name, b.Name)
	})
	return rules
}

func TestSpecification_IsComplete(t *testing.T) {
	requireZ3(t)
	model, err := CheckCompleteness(getSpecificationRules())
	require.NoError(t, err)
	if model != "" {
		t.Errorf("no rule matches the state\n%s", model)
	}
}

func TestSpecification_IsSound(t *testing.T) {
	requireZ3(t)
	conflicts, err := CheckSoundness(getSpecificationRules(), 20)
	require.NoError(t, err)
	for _, conflict := range conflicts {
		t.Errorf("rules %s and %s have different effects but match the state\n%s",
			conflict.First.Name, conflict.Second.Name, conflict.Model)
	}
}
