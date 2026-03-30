// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

// Package smt defines interfaces for building SMT solver terms used
// in the specification prover. The concrete CVC5 implementation lives
// in the cvc5 subpackage.
package smt

// Term represents an opaque SMT solver term.
// Concrete implementations are provided by solver-specific packages.
type Term interface{}

// Context provides methods to create SMT terms representing
// the symbolic VM state and predicate abstractions.
type Context interface {
	// Core term constructors
	IntConst(val int64) Term
	IntConstStr(val string) Term
	BoolConst(name string) Term
	IntVar(name string) Term
	True() Term
	False() Term

	// Logical operations
	And(children ...Term) Term
	Or(children ...Term) Term
	Not(child Term) Term
	Eq(lhs, rhs Term) Term
	Lt(lhs, rhs Term) Term
	Leq(lhs, rhs Term) Term
	Gt(lhs, rhs Term) Term
	Geq(lhs, rhs Term) Term
	Implies(lhs, rhs Term) Term

	// VM state variables
	StatusTerm() Term
	PcTerm() Term
	GasTerm() Term
	StackSizeTerm() Term
	RevisionTerm() Term
	ReadOnlyTerm() Term
	SelfDestructedTerm() Term

	// Stack and code
	Param(pos int) Term
	Code(x Term) Term

	// Predicate abstractions
	AccountCold(x string) Term
	AccountWarm(x string) Term
	AccountEmpty(x string) Term
	Balance(x string) Term
	StorageCold(x string) Term
	StorageConf(status int64, key, newValue string) Term
	TranStorageNonZero(x string) Term
	TranStorageToZero(x string) Term
	HasBlobHash(x string) Term
	InRange256FromCurrentBlock(x string) Term
	IsCode(x string) Term
	IsData(x string) Term

	// Delegation designation
	NoDelegationDesignation(x string) Term
	ColdDelegationDesignation(x string) Term
	WarmDelegationDesignation(x string) Term

	// Solver operations
	// CheckSatWith creates a solver with VM state constraints,
	// asserts the given formula, and checks satisfiability.
	// Returns (isSat, isConclusive).
	CheckSatWith(formula Term) (bool, bool)
}
