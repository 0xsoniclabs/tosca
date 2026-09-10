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
	"fmt"
	"math/big"
	"strconv"
	"strings"

	. "github.com/0xsoniclabs/tosca/go/ct/common"
	"github.com/0xsoniclabs/tosca/go/ct/st"
	"github.com/0xsoniclabs/tosca/go/tosca"
)

// This file declares the abstract SMT model of the EVM state and the helpers
// used by the ToSmt methods of conditions and expressions to encode them as
// SMT-LIB terms over that model. Values are integers, state properties
// depending on an argument (code bytes, stack elements, account and storage
// properties) are uninterpreted functions. A condition whose evaluation fails
// on a state (e.g. a stack parameter of a too small stack) does not match that
// state; such terms therefore carry guards that are conjoined to every
// condition using them.

// SmtStateModel declares the symbols used by the SMT encodings of conditions
// and constrains them to their domains.
func SmtStateModel() string {
	return fmt.Sprintf(`
(declare-const status Int)
(declare-const revision Int)
(declare-const pc Int)
(declare-const gas Int)
(declare-const stackSize Int)
(declare-const readOnly Bool)
(declare-const selfAddress Int)
(declare-const isNewContract Bool)
(declare-const hasSelfDestructed Bool)
(declare-fun code (Int) Int)
(declare-fun isCode (Int) Bool)
(declare-fun param (Int) Int)
(declare-fun toAddress (Int) Int)
(declare-fun balance (Int) Int)
(declare-fun accountEmpty (Int) Bool)
(declare-fun accountWarm (Int) Bool)
(declare-fun storageWarm (Int) Bool)
(declare-fun storageStatus (Int Int) Int)
(declare-fun transientStorageIsZero (Int) Bool)
(declare-fun hasBlobHash (Int) Bool)
(declare-fun inRange256FromCurrentBlock (Int) Bool)
(declare-fun delegationDesignation (Int) Int)

(assert (and (<= 0 status) (< status %d)))
(assert (and (<= %d revision) (<= revision %d)))
(assert (<= 0 pc))
(assert (<= 0 gas))
(assert (and (<= 0 stackSize) (<= stackSize %d)))
(assert (forall ((p Int)) (and (<= 0 (code p)) (<= (code p) 255))))
(assert (forall ((i Int)) (<= 0 (param i))))
(assert (forall ((a Int)) (<= 0 (balance a))))
(assert (forall ((k Int) (v Int)) (and (<= 0 (storageStatus k v)) (<= (storageStatus k v) %d))))
(assert (forall ((a Int)) (and (<= 0 (delegationDesignation a)) (<= (delegationDesignation a) %d))))

; Storage statuses beyond StorageModified have a current value different from
; the original one, so the slot was written in this transaction and is warm.
(assert (forall ((k Int) (v Int)) (=> (> (storageStatus k v) %d) (storageWarm k))))
`,
		st.NumStatusCodes,
		MinRevision, NewestSupportedRevision,
		st.MaxStackSize,
		tosca.StorageModifiedRestored,
		ColdDelegationDesignation,
		tosca.StorageModified,
	)
}

// SmtTerm is an SMT-LIB term together with the guards under which the
// evaluation of the encoded expression does not fail.
type SmtTerm struct {
	Term   string
	Guards []string
}

func smtLiteral(value any) string {
	switch v := value.(type) {
	case U256:
		return v.DecimalString()
	case tosca.Address:
		return new(big.Int).SetBytes(v[:]).String()
	case bool:
		return strconv.FormatBool(v)
	}
	literal := fmt.Sprintf("%d", value)
	if negative, found := strings.CutPrefix(literal, "-"); found {
		return fmt.Sprintf("(- %s)", negative)
	}
	return literal
}

func smtApply(function string, args ...SmtTerm) SmtTerm {
	res := SmtTerm{Term: function}
	for _, arg := range args {
		res.Term += " " + arg.Term
		res.Guards = append(res.Guards, arg.Guards...)
	}
	res.Term = "(" + res.Term + ")"
	return res
}

func smtGuarded(guards []string, condition string) string {
	if len(guards) == 0 {
		return condition
	}
	return fmt.Sprintf("(and %s %s)", strings.Join(guards, " "), condition)
}

func smtPredicate(predicate string, args ...SmtTerm) string {
	application := smtApply(predicate, args...)
	return smtGuarded(application.Guards, application.Term)
}

func smtNegatedPredicate(predicate string, args ...SmtTerm) string {
	application := smtApply(predicate, args...)
	return smtGuarded(application.Guards, "(not "+application.Term+")")
}

func smtCompare(operator string, lhs SmtTerm, rhs any) string {
	return smtGuarded(lhs.Guards, fmt.Sprintf("(%s %s %s)", operator, lhs.Term, smtLiteral(rhs)))
}

func smtEnumerated[T ~uint64 | ~int](function string, value T, args ...SmtTerm) string {
	application := smtApply(function, args...)
	return smtGuarded(application.Guards, fmt.Sprintf("(= %s %d)", application.Term, value))
}
