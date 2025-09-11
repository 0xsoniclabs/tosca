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
	name string
	fun  func(*st.State)
}

func Change(name string, fun func(*st.State)) Effect {
	return &change{name, fun}
}

func (c *change) Apply(state *st.State) {
	c.fun(state)
}

func (c *change) String() string {
	return c.name
}

////////////////////////////////////////////////////////////

func NoEffect() Effect {
	return Change("NoEffect", func(*st.State) {})
}

func FailEffect() Effect {
	return Change("FailEffect", func(s *st.State) {
		s.Status = st.Failed
		s.Gas = 0
	})
}
