// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package rlz

import (
	"regexp"
	"strings"
	"testing"

	. "github.com/0xsoniclabs/tosca/go/ct/common"
	"github.com/0xsoniclabs/tosca/go/ct/st"
	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/0xsoniclabs/tosca/go/tosca/vm"
)

var conditionEncodings = map[string]Condition{
	"true":                                         And(),
	"(and (= status 0) (>= gas 3))":                And(Eq(Status(), st.Running), Ge(Gas(), 3)),
	"(< gas (- 1))":                                Lt(Gas(), -1),
	"(<= stackSize 1024)":                          Le(StackSize(), 1024),
	"(= readOnly true)":                            Eq(ReadOnly(), true),
	"(= selfAddress 1)":                            Eq(SelfAddress(), tosca.Address{19: 1}),
	"(> (balance selfAddress) 0)":                  Gt(Balance(SelfAddress()), NewU256(0)),
	"(and (<= 1 revision) (<= revision 2))":        RevisionBounds(tosca.R09_Berlin, tosca.R10_London),
	"(and (isCode pc) (= (code pc) 1))":            Eq(Op(Pc()), vm.ADD),
	"(not (isCode pc))":                            IsData(Pc()),
	"(and (< 1 stackSize) (distinct (param 1) 0))": Ne(Param(1), NewU256(0)),
	"(and (< 0 stackSize) (isCode (param 0)) (distinct (code (param 0)) 91))":         Ne(Op(Param(0)), vm.JUMPDEST),
	"(and (< 0 stackSize) (= (balance (toAddress (param 0))) 20))":                    Eq(Balance(ToAddress(Param(0))), NewU256(20)),
	"(and (< 0 stackSize) (not (storageWarm (param 0))))":                             IsStorageCold(Param(0)),
	"(and (< 0 stackSize) (< 1 stackSize) (= (storageStatus (param 0) (param 1)) 2))": StorageConfiguration(tosca.StorageDeleted, Param(0), Param(1)),
	"(and (< 0 stackSize) (transientStorageIsZero (param 0)))":                        BindTransientStorageToZero(Param(0)),
	"(and (< 0 stackSize) (not (accountEmpty (param 0))))":                            AccountIsNotEmpty(Param(0)),
	"(and (< 1 stackSize) (accountWarm (param 1)))":                                   IsAddressWarm(Param(1)),
	"(not isNewContract)": IsNotNewContract(),
	"hasSelfDestructed":   HasSelfDestructed(),
	"(and (< 0 stackSize) (inRange256FromCurrentBlock (param 0)))":  InRange256FromCurrentBlock(Param(0)),
	"(and (< 0 stackSize) (not (hasBlobHash (param 0))))":           HasNoBlobHash(Param(0)),
	"(and (< 1 stackSize) (= (delegationDesignation (param 1)) 2))": ConstraintDelegationDesignator(Param(1), ColdDelegationDesignation),
}

func TestToSmt_EncodesConditions(t *testing.T) {
	for want, condition := range conditionEncodings {
		if got := condition.ToSmt(); got != want {
			t.Errorf("unexpected encoding of %v: want %s, got %s", condition, want, got)
		}
	}
}

func TestSmtStateModel_IsWellFormed(t *testing.T) {
	model := SmtStateModel()
	if strings.Contains(model, "%") {
		t.Errorf("state model contains unresolved format verbs:\n%s", model)
	}
	if strings.Count(model, "(") != strings.Count(model, ")") {
		t.Errorf("state model has unbalanced parentheses:\n%s", model)
	}
}

func TestSmtStateModel_DeclaresSymbolsUsedByEncodings(t *testing.T) {
	declared := map[string]bool{"and": true, "or": true, "not": true, "distinct": true, "true": true, "false": true}
	for _, match := range regexp.MustCompile(`\(declare-(?:const|fun) (\w+)`).FindAllStringSubmatch(SmtStateModel(), -1) {
		declared[match[1]] = true
	}
	symbols := regexp.MustCompile(`[A-Za-z]\w*`)
	for encoding, condition := range conditionEncodings {
		for _, symbol := range symbols.FindAllString(encoding, -1) {
			if !declared[symbol] {
				t.Errorf("encoding of %v uses undeclared symbol %s", condition, symbol)
			}
		}
	}
}
