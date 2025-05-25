// Copyright (c) 2024 Fantom Foundation
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at fantom.foundation/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package rlz

import (
	"fmt"
	"strconv"

	"github.com/Fantom-foundation/Tosca/go/ct/common"
	"github.com/Fantom-foundation/Tosca/go/ct/st"
	"github.com/Fantom-foundation/Tosca/go/tosca"
)

type Effect interface {
	// Apply modifies the given state with this effect.
	Apply(*st.State)

	fmt.Stringer
}

////////////////////////////////////////////////////////////
// Change

type change struct {
	fun func(*st.State)
}

func Change(fun func(*st.State)) Effect {
	return &change{fun}
}

func (c *change) Apply(state *st.State) {
	c.fun(state)
}

func (c *change) String() string {
	return "change"
}

////////////////////////////////////////////////////////////
// FChange as a functional side-effect

type FChange struct { // TODO: make it private after everything is a function term
	fun F
	Effect
}

func NewFChange(fun F) Effect {
	return &FChange{fun: fun}
}

func (c FChange) Apply(state *st.State) {
	if r := c.fun.Apply(state); r != nil {
		panic("Function return a result")
	}
}

func (c FChange) String() string {
	return c.fun.String()
}

func (c FChange) Get() F {
	return c.fun
}

////////////////////////////////////////////////////////////
// Function

type F interface {
	Apply(state *st.State) F
	fmt.Stringer
}

////////////////////////////////////////////////////////////
// UINT256 Constant Function

type FConstU256 struct {
	value common.U256 // TODO: check whether storing pointer is faster
	F
}

func NewFConstU256(value *common.U256) *FConstU256 {
	return &FConstU256{value: *value}
}

func (f FConstU256) Apply(state *st.State) F {
	return f
}

func (f FConstU256) Get() common.U256 {
	return f.value
}

func (f FConstU256) String() string {
	return "U256(" + f.value.String() + ")"
}

////////////////////////////////////////////////////////////
// UINT256 Constant Function

type FSeq struct {
	stmts []F
	F
}

func NewFSeq(stmts ...F) *FSeq {
	return &FSeq{stmts: stmts}
}

func (f FSeq) Apply(state *st.State) F {
	for _, stmt := range f.stmts {
		if result := stmt.Apply(state); result != nil {
			panic("Statement return a non-nil value")
		}
	}
	return nil
}

func (f FSeq) String() string {
	sz := len(f.stmts)
	if sz > 0 {
		str := f.stmts[0].String()
		for i := 1; i < sz; i++ {
			str += ";" + f.stmts[i].String()
		}
		return str
	} else {
		return ""
	}
}

////////////////////////////////////////////////////////////
// State getter for U256

type GetterU256 func(*st.State) common.U256

type FGetStateU256 struct {
	name string
	fun  GetterU256
	F
}

func NewFGetStateU256(name string, fun GetterU256) *FGetStateU256 {
	return &FGetStateU256{name: name, fun: fun}
}

func (f *FGetStateU256) Apply(state *st.State) F {
	result := f.fun(state)
	return NewFConstU256(&result)
}

func (f *FGetStateU256) String() string {
	return f.name
}

// state specificy getter
func NewFPeekStack(pos int) *FGetStateU256 {
	return NewFGetStateU256("peek-"+strconv.Itoa(pos), func(state *st.State) common.U256 {
		return state.Stack.Get(pos)
	})
}

////////////////////////////////////////////////////////////
// State setter / no functional arguments

type Setter func(*st.State)

type FSetState struct {
	name string
	fun  Setter
	F
}

func NewFSetState(name string, fun Setter) *FSetState {
	return &FSetState{name: name, fun: fun}
}

func (f *FSetState) Apply(state *st.State) F {
	f.fun(state)
	return nil
}

func (f *FSetState) String() string {
	return f.name
}

// state specific setters

// increment pc in state
func NewFIncPC() *FSetState {
	return NewFSetState("inc-pc", func(state *st.State) { state.Pc++ })
}

// decrement gas by constant "gas"
func NewFDecGas(gas tosca.Gas) *FSetState {
	return NewFSetState("dec-gas-"+strconv.FormatInt(int64(gas), 10), func(state *st.State) { state.Gas -= gas })
}

////////////////////////////////////////////////////////////
// State setter for U256

type SetterU256 func(*st.State, common.U256)

type FSetStateU256 struct {
	name string
	fun  SetterU256
	op   F
	F
}

func NewFSetStateU256(name string, fun SetterU256, op F) *FSetStateU256 {
	return &FSetStateU256{name: name, fun: fun, op: op}
}

func (f *FSetStateU256) Apply(state *st.State) F {
	var constOp FConstU256
	var ok bool
	fOp := f.op.Apply(state)
	if constOp, ok = fOp.(FConstU256); !ok {
		panic("Expected U256 constant in parameter")
	}
	f.fun(state, constOp.Get())
	return nil
}

func (f *FSetStateU256) String() string {
	return f.name + "(" + f.op.String() + ")"
}

// state specificy getter

// pop "num" elements form stack and push an argument (as a function) onto the stack
func NewFPopPush(num int, op F) *FSetStateU256 {
	return NewFSetStateU256("pop-"+strconv.Itoa(num)+"-push", func(state *st.State, value common.U256) {
		state.Stack.Resize(state.Stack.Size() - 2)
		state.Stack.Push(value)
	}, op)
}

////////////////////////////////////////////////////////////
// Binary Function U256

type BinaryOpU256 func(common.U256, common.U256) common.U256

type FBinaryU256 struct {
	name        string
	fun         BinaryOpU256
	left, right F
	F
}

func NewFBinaryU256(name string, fun BinaryOpU256, left, right F) *FBinaryU256 {
	return &FBinaryU256{name: name, fun: fun, left: left, right: right}
}

func (f *FBinaryU256) Apply(state *st.State) F {
	var constLeft, constRight FConstU256
	var ok bool
	fLeft := f.left.Apply(state)
	fRight := f.right.Apply(state)
	if constLeft, ok = fLeft.(FConstU256); !ok {
		panic("Expected U256 constant in left parameter")
	}
	if constRight, ok = fRight.(FConstU256); !ok {
		panic("Expected U256 constant in right parameter")
	}
	result := f.fun(constLeft.Get(), constRight.Get())
	return NewFConstU256(&result)
}

func (f *FBinaryU256) String() string {
	return f.name + "(" + f.left.String() + "," + f.right.String() + ")"
}

// //////////////////////////////////////////////////////////
func NoEffect() Effect {
	return Change(func(*st.State) {})
}

func FailEffect() Effect {
	return Change(func(s *st.State) {
		s.Status = st.Failed
		s.Gas = 0
	})
}
