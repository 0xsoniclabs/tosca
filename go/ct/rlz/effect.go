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

	"github.com/0xsoniclabs/tosca/go/ct/st"
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

type noEffect struct{}

// NoEffect returns an effect leaving the state unchanged. The returned effect
// is comparable, so that rules without an effect can be identified.
func NoEffect() Effect {
	return noEffect{}
}

func (noEffect) Apply(*st.State) {}

func (noEffect) String() string {
	return "none"
}

type failEffect struct{}

// FailEffect returns the effect of a failing instruction. The returned effect
// is comparable, so that rules with a failing effect can be identified.
func FailEffect() Effect {
	return failEffect{}
}

func (failEffect) Apply(s *st.State) {
	s.Status = st.Failed
	s.Gas = 0
}

func (failEffect) String() string {
	return "fail"
}
