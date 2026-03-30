// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

// Package cvc5 provides a CVC5-based implementation of the smt.Context
// interface for SMT-based specification verification.
package cvc5

// #cgo CFLAGS: -I/opt/homebrew/Caskroom/cvc5/1.3.3/cvc5-macOS-arm64-static/include
// #cgo LDFLAGS: -L/opt/homebrew/Caskroom/cvc5/1.3.3/cvc5-macOS-arm64-static/lib -lcvc5 -lcadical -lgmp -lpicpoly -lpicpolyxx -lc++
// #include <cvc5/c/cvc5.h>
// #include <stdlib.h>
import "C"
import "unsafe"

// term wraps a CVC5 term pointer and implements smt.Term.
type term struct {
	c C.Cvc5Term
}

// sort wraps a CVC5 sort pointer.
type sort struct {
	c C.Cvc5Sort
}

// termManager wraps the CVC5 term manager.
type termManager struct {
	c *C.Cvc5TermManager
}

func newTermManager() *termManager {
	return &termManager{c: C.cvc5_term_manager_new()}
}

func (tm *termManager) delete() {
	if tm.c != nil {
		C.cvc5_term_manager_delete(tm.c)
		tm.c = nil
	}
}

func (tm *termManager) integerSort() sort {
	return sort{c: C.cvc5_get_integer_sort(tm.c)}
}

func (tm *termManager) booleanSort() sort {
	return sort{c: C.cvc5_get_boolean_sort(tm.c)}
}

func (tm *termManager) arraySort(index, elem sort) sort {
	return sort{c: C.cvc5_mk_array_sort(tm.c, index.c, elem.c)}
}

func (tm *termManager) mkConst(s sort, name string) term {
	cName := C.CString(name)
	defer C.free(unsafe.Pointer(cName))
	return term{c: C.cvc5_mk_const(tm.c, s.c, cName)}
}

func (tm *termManager) mkInteger(val int64) term {
	return term{c: C.cvc5_mk_integer_int64(tm.c, C.int64_t(val))}
}

func (tm *termManager) mkIntegerStr(val string) term {
	cVal := C.CString(val)
	defer C.free(unsafe.Pointer(cVal))
	return term{c: C.cvc5_mk_integer(tm.c, cVal)}
}

func (tm *termManager) mkTrue() term {
	return term{c: C.cvc5_mk_true(tm.c)}
}

func (tm *termManager) mkFalse() term {
	return term{c: C.cvc5_mk_false(tm.c)}
}

func (tm *termManager) mkTerm(kind C.Cvc5Kind, children ...term) term {
	n := len(children)
	if n == 0 {
		return term{c: C.cvc5_mk_term(tm.c, kind, 0, nil)}
	}
	cChildren := make([]C.Cvc5Term, n)
	for i, child := range children {
		cChildren[i] = child.c
	}
	return term{c: C.cvc5_mk_term(tm.c, kind, C.size_t(n), &cChildren[0])}
}

// solver wraps the CVC5 solver.
type solver struct {
	c  *C.Cvc5
	tm *termManager
}

func newSolver(tm *termManager) *solver {
	return &solver{
		c:  C.cvc5_new(tm.c),
		tm: tm,
	}
}

func (s *solver) delete() {
	if s.c != nil {
		C.cvc5_delete(s.c)
		s.c = nil
	}
}

func (s *solver) setLogic(logic string) {
	cLogic := C.CString(logic)
	defer C.free(unsafe.Pointer(cLogic))
	C.cvc5_set_logic(s.c, cLogic)
}

func (s *solver) assert(t term) {
	C.cvc5_assert_formula(s.c, t.c)
}

// checkSat checks satisfiability.
// Returns (isSat, isConclusive).
func (s *solver) checkSat() (bool, bool) {
	result := C.cvc5_check_sat(s.c)
	if C.cvc5_result_is_sat(result) {
		return true, true
	}
	if C.cvc5_result_is_unsat(result) {
		return false, true
	}
	return false, false
}
