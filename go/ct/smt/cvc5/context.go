// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package cvc5

// #include <cvc5/c/cvc5.h>
import "C"

import (
	"fmt"

	smtpkg "github.com/0xsoniclabs/tosca/go/ct/smt"
)

// context implements smt.Context using CVC5.
type context struct {
	tm *termManager

	// Core symbolic state variables
	status         term
	pc             term
	gas            term
	stackSize      term
	revision       term
	stack          term
	codeBlock      term
	readOnly       term
	selfDestructed term

	// Sorts
	intSort  sort
	boolSort sort

	// Caches for predicate abstractions
	accountColdCache    map[string]term
	accountEmptyCache   map[string]term
	balanceCache        map[string]term
	storageColdCache    map[string]term
	storageConfCache    map[string]term
	tranStorageCache    map[string]term
	blobHashCache       map[string]term
	inRangeCache        map[string]term
	isCodeCache         map[string]term
	delegDesigCache     map[string]term
	coldDelegDesigCache map[string]term
}

// NewContext creates a new CVC5-based SMT context.
func NewContext() smtpkg.Context {
	tm := newTermManager()
	intSort := tm.integerSort()
	boolSort := tm.booleanSort()
	arraySort := tm.arraySort(intSort, intSort)

	return &context{
		tm:                  tm,
		status:              tm.mkConst(intSort, "status"),
		pc:                  tm.mkConst(intSort, "pc"),
		gas:                 tm.mkConst(intSort, "gas"),
		stackSize:           tm.mkConst(intSort, "stackSize"),
		revision:            tm.mkConst(intSort, "revision"),
		stack:               tm.mkConst(arraySort, "stack"),
		codeBlock:           tm.mkConst(arraySort, "code_block"),
		readOnly:            tm.mkConst(boolSort, "readOnly"),
		selfDestructed:      tm.mkConst(boolSort, "hasSelfDestructed"),
		intSort:             intSort,
		boolSort:            boolSort,
		accountColdCache:    make(map[string]term),
		accountEmptyCache:   make(map[string]term),
		balanceCache:        make(map[string]term),
		storageColdCache:    make(map[string]term),
		storageConfCache:    make(map[string]term),
		tranStorageCache:    make(map[string]term),
		blobHashCache:       make(map[string]term),
		inRangeCache:        make(map[string]term),
		isCodeCache:         make(map[string]term),
		delegDesigCache:     make(map[string]term),
		coldDelegDesigCache: make(map[string]term),
	}
}

// Delete frees all CVC5 resources.
func (ctx *context) Delete() {
	ctx.tm.delete()
}

// asTerm converts smt.Term to the internal term type.
func asTerm(t smtpkg.Term) term {
	return t.(term)
}

func (ctx *context) IntConst(val int64) smtpkg.Term {
	return ctx.tm.mkInteger(val)
}

func (ctx *context) IntConstStr(val string) smtpkg.Term {
	return ctx.tm.mkIntegerStr(val)
}

func (ctx *context) BoolConst(name string) smtpkg.Term {
	return ctx.tm.mkConst(ctx.boolSort, name)
}

func (ctx *context) IntVar(name string) smtpkg.Term {
	return ctx.tm.mkConst(ctx.intSort, name)
}

func (ctx *context) True() smtpkg.Term {
	return ctx.tm.mkTrue()
}

func (ctx *context) False() smtpkg.Term {
	return ctx.tm.mkFalse()
}

func (ctx *context) And(children ...smtpkg.Term) smtpkg.Term {
	switch len(children) {
	case 0:
		return ctx.tm.mkTrue()
	case 1:
		return children[0]
	default:
		ts := make([]term, len(children))
		for i, c := range children {
			ts[i] = asTerm(c)
		}
		return ctx.tm.mkTerm(C.CVC5_KIND_AND, ts...)
	}
}

func (ctx *context) Or(children ...smtpkg.Term) smtpkg.Term {
	switch len(children) {
	case 0:
		return ctx.tm.mkFalse()
	case 1:
		return children[0]
	default:
		ts := make([]term, len(children))
		for i, c := range children {
			ts[i] = asTerm(c)
		}
		return ctx.tm.mkTerm(C.CVC5_KIND_OR, ts...)
	}
}

func (ctx *context) Not(child smtpkg.Term) smtpkg.Term {
	return ctx.tm.mkTerm(C.CVC5_KIND_NOT, asTerm(child))
}

func (ctx *context) Eq(lhs, rhs smtpkg.Term) smtpkg.Term {
	return ctx.tm.mkTerm(C.CVC5_KIND_EQUAL, asTerm(lhs), asTerm(rhs))
}

func (ctx *context) Lt(lhs, rhs smtpkg.Term) smtpkg.Term {
	return ctx.tm.mkTerm(C.CVC5_KIND_LT, asTerm(lhs), asTerm(rhs))
}

func (ctx *context) Leq(lhs, rhs smtpkg.Term) smtpkg.Term {
	return ctx.tm.mkTerm(C.CVC5_KIND_LEQ, asTerm(lhs), asTerm(rhs))
}

func (ctx *context) Gt(lhs, rhs smtpkg.Term) smtpkg.Term {
	return ctx.tm.mkTerm(C.CVC5_KIND_GT, asTerm(lhs), asTerm(rhs))
}

func (ctx *context) Geq(lhs, rhs smtpkg.Term) smtpkg.Term {
	return ctx.tm.mkTerm(C.CVC5_KIND_GEQ, asTerm(lhs), asTerm(rhs))
}

func (ctx *context) Implies(lhs, rhs smtpkg.Term) smtpkg.Term {
	return ctx.tm.mkTerm(C.CVC5_KIND_IMPLIES, asTerm(lhs), asTerm(rhs))
}

func (ctx *context) StatusTerm() smtpkg.Term         { return ctx.status }
func (ctx *context) PcTerm() smtpkg.Term             { return ctx.pc }
func (ctx *context) GasTerm() smtpkg.Term            { return ctx.gas }
func (ctx *context) StackSizeTerm() smtpkg.Term      { return ctx.stackSize }
func (ctx *context) RevisionTerm() smtpkg.Term       { return ctx.revision }
func (ctx *context) ReadOnlyTerm() smtpkg.Term       { return ctx.readOnly }
func (ctx *context) SelfDestructedTerm() smtpkg.Term { return ctx.selfDestructed }

func (ctx *context) Param(pos int) smtpkg.Term {
	index := ctx.tm.mkTerm(C.CVC5_KIND_SUB, ctx.stackSize, ctx.tm.mkInteger(int64(pos+1)))
	return ctx.tm.mkTerm(C.CVC5_KIND_SELECT, ctx.stack, index)
}

func (ctx *context) Code(x smtpkg.Term) smtpkg.Term {
	return ctx.tm.mkTerm(C.CVC5_KIND_SELECT, ctx.codeBlock, asTerm(x))
}

func (ctx *context) AccountCold(x string) smtpkg.Term {
	return ctx.getCachedBool(ctx.accountColdCache, "cold_account_", x)
}

func (ctx *context) AccountWarm(x string) smtpkg.Term {
	return ctx.Not(ctx.AccountCold(x))
}

func (ctx *context) AccountEmpty(x string) smtpkg.Term {
	return ctx.getCachedBool(ctx.accountEmptyCache, "account_empty_", x)
}

func (ctx *context) Balance(x string) smtpkg.Term {
	return ctx.getCachedInt(ctx.balanceCache, "balance_", x)
}

func (ctx *context) StorageCold(x string) smtpkg.Term {
	return ctx.getCachedBool(ctx.storageColdCache, "storage_cold_", x)
}

func (ctx *context) StorageConf(status int64, key, newValue string) smtpkg.Term {
	cacheKey := key + "_" + newValue
	var confVar term
	if t, ok := ctx.storageConfCache[cacheKey]; ok {
		confVar = t
	} else {
		confVar = ctx.tm.mkConst(ctx.intSort, "storageConf_"+cacheKey)
		ctx.storageConfCache[cacheKey] = confVar
	}
	return ctx.tm.mkTerm(C.CVC5_KIND_EQUAL, confVar, ctx.tm.mkInteger(status))
}

func (ctx *context) TranStorageNonZero(x string) smtpkg.Term {
	return ctx.getCachedBool(ctx.tranStorageCache, "tran_storage_", x)
}

func (ctx *context) TranStorageToZero(x string) smtpkg.Term {
	return ctx.Not(ctx.TranStorageNonZero(x))
}

func (ctx *context) HasBlobHash(x string) smtpkg.Term {
	return ctx.getCachedBool(ctx.blobHashCache, "has_blob_", x)
}

func (ctx *context) InRange256FromCurrentBlock(x string) smtpkg.Term {
	return ctx.getCachedBool(ctx.inRangeCache, "inRange_", x)
}

func (ctx *context) IsCode(x string) smtpkg.Term {
	return ctx.getCachedBool(ctx.isCodeCache, "is_code_", x)
}

func (ctx *context) IsData(x string) smtpkg.Term {
	return ctx.Not(ctx.IsCode(x))
}

func (ctx *context) NoDelegationDesignation(x string) smtpkg.Term {
	return ctx.Not(ctx.getDelegDesig(x))
}

func (ctx *context) ColdDelegationDesignation(x string) smtpkg.Term {
	return ctx.And(ctx.getDelegDesig(x), ctx.getColdDelegDesig(x))
}

func (ctx *context) WarmDelegationDesignation(x string) smtpkg.Term {
	return ctx.And(ctx.getDelegDesig(x), ctx.Not(ctx.getColdDelegDesig(x)))
}

func (ctx *context) getDelegDesig(x string) smtpkg.Term {
	return ctx.getCachedBool(ctx.delegDesigCache, "deleg_desig_", x)
}

func (ctx *context) getColdDelegDesig(x string) smtpkg.Term {
	return ctx.getCachedBool(ctx.coldDelegDesigCache, "cold_deleg_", x)
}

// getCachedBool returns a cached boolean variable, creating it if needed.
func (ctx *context) getCachedBool(cache map[string]term, prefix, key string) smtpkg.Term {
	if t, ok := cache[key]; ok {
		return t
	}
	t := ctx.tm.mkConst(ctx.boolSort, prefix+key)
	cache[key] = t
	return t
}

// getCachedInt returns a cached integer variable, creating it if needed.
func (ctx *context) getCachedInt(cache map[string]term, prefix, key string) smtpkg.Term {
	if t, ok := cache[key]; ok {
		return t
	}
	t := ctx.tm.mkConst(ctx.intSort, prefix+key)
	cache[key] = t
	return t
}

// vmStateConstraints returns the conjunction of all valid VM state constraints.
func (ctx *context) vmStateConstraints() term {
	zero := ctx.tm.mkInteger(0)

	constraints := []term{
		// revision bounds
		ctx.tm.mkTerm(C.CVC5_KIND_GEQ, ctx.revision, zero),
		ctx.tm.mkTerm(C.CVC5_KIND_LT, ctx.revision, ctx.tm.mkInteger(smtpkg.NumRevisions)),
		// status bounds
		ctx.tm.mkTerm(C.CVC5_KIND_GEQ, ctx.status, zero),
		ctx.tm.mkTerm(C.CVC5_KIND_LT, ctx.status, ctx.tm.mkInteger(smtpkg.NumStatusCodes)),
		// pc bounds
		ctx.tm.mkTerm(C.CVC5_KIND_GEQ, ctx.pc, zero),
		ctx.tm.mkTerm(C.CVC5_KIND_LT, ctx.pc, ctx.tm.mkInteger(smtpkg.MaxCodeSize)),
		// gas bounds
		ctx.tm.mkTerm(C.CVC5_KIND_GEQ, ctx.gas, zero),
		// stack size bounds
		ctx.tm.mkTerm(C.CVC5_KIND_GEQ, ctx.stackSize, zero),
		ctx.tm.mkTerm(C.CVC5_KIND_LEQ, ctx.stackSize, ctx.tm.mkInteger(smtpkg.MaxStackSize)),
		// code(pc) in byte range
		ctx.tm.mkTerm(C.CVC5_KIND_GEQ, ctx.tm.mkTerm(C.CVC5_KIND_SELECT, ctx.codeBlock, ctx.pc), zero),
		ctx.tm.mkTerm(C.CVC5_KIND_LEQ, ctx.tm.mkTerm(C.CVC5_KIND_SELECT, ctx.codeBlock, ctx.pc), ctx.tm.mkInteger(255)),
	}

	// Balance bounds for all cached balance variables
	for _, bal := range ctx.balanceCache {
		constraints = append(constraints, ctx.tm.mkTerm(C.CVC5_KIND_GEQ, bal, zero))
	}

	// Storage configuration bounds
	for _, conf := range ctx.storageConfCache {
		constraints = append(constraints, ctx.tm.mkTerm(C.CVC5_KIND_GEQ, conf, zero))
		constraints = append(constraints, ctx.tm.mkTerm(C.CVC5_KIND_LT, conf, ctx.tm.mkInteger(smtpkg.NumStorageStatus)))
	}

	// Storage cold implies non-assigned configuration
	warmOnlyConfigs := []int64{
		smtpkg.StorageAssigned,
		smtpkg.StorageAddedDeleted,
		smtpkg.StorageDeletedRestored,
		smtpkg.StorageDeletedAdded,
		smtpkg.StorageModifiedDeleted,
		smtpkg.StorageModifiedRestored,
	}
	for _, cold := range ctx.storageColdCache {
		for _, conf := range ctx.storageConfCache {
			for _, cfg := range warmOnlyConfigs {
				constraints = append(constraints,
					ctx.tm.mkTerm(C.CVC5_KIND_IMPLIES,
						ctx.tm.mkTerm(C.CVC5_KIND_EQUAL, conf, ctx.tm.mkInteger(cfg)),
						ctx.tm.mkTerm(C.CVC5_KIND_NOT, cold),
					),
				)
			}
		}
	}

	// Combine all constraints
	if len(constraints) == 1 {
		return constraints[0]
	}
	return ctx.tm.mkTerm(C.CVC5_KIND_AND, constraints...)
}

// CheckSatWith creates a solver with VM state constraints,
// asserts the given formula, and checks satisfiability.
func (ctx *context) CheckSatWith(formula smtpkg.Term) (bool, bool) {
	s := newSolver(ctx.tm)
	defer s.delete()
	s.setLogic("ALL")
	s.assert(ctx.vmStateConstraints())
	s.assert(asTerm(formula))
	return s.checkSat()
}

// String implements fmt.Stringer for debugging.
func (ctx *context) String() string {
	return fmt.Sprintf("cvc5.Context{balances:%d, storageCold:%d, storageConf:%d}",
		len(ctx.balanceCache), len(ctx.storageColdCache), len(ctx.storageConfCache))
}
