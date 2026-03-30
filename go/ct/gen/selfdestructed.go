// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package gen

import (
	"strings"

	"pgregory.net/rand"
)

type SelfDestructedGenerator struct {
	mustBeNewContract       bool
	mustNotBeNewContract    bool
	mustBeSelfDestructed    bool
	mustNotBeSelfDestructed bool
}

func NewSelfDestructedGenerator() *SelfDestructedGenerator {
	return &SelfDestructedGenerator{}
}

func (g *SelfDestructedGenerator) Clone() *SelfDestructedGenerator {
	return &SelfDestructedGenerator{
		mustBeNewContract:       g.mustBeNewContract,
		mustNotBeNewContract:    g.mustNotBeNewContract,
		mustBeSelfDestructed:    g.mustBeSelfDestructed,
		mustNotBeSelfDestructed: g.mustNotBeSelfDestructed,
	}
}

func (g *SelfDestructedGenerator) Restore(other *SelfDestructedGenerator) {
	if g == other {
		return
	}
	*g = *other
}

func (g *SelfDestructedGenerator) MarkAsNewContract() {
	g.mustBeNewContract = true
}

func (g *SelfDestructedGenerator) MarkAsNotNewContract() {
	g.mustNotBeNewContract = true
}

func (g *SelfDestructedGenerator) MarkAsSelfDestructed() {
	g.mustBeSelfDestructed = true
}

func (g *SelfDestructedGenerator) MarkAsNotSelfDestructed() {
	g.mustNotBeSelfDestructed = true
}

func (g *SelfDestructedGenerator) String() string {
	if g.mustBeSelfDestructed && g.mustNotBeSelfDestructed ||
		g.mustBeNewContract && g.mustNotBeNewContract {
		return "{false}" // unsatisfiable
	}
	if !g.mustBeSelfDestructed && !g.mustNotBeSelfDestructed &&
		!g.mustBeNewContract && !g.mustNotBeNewContract {
		return "{true}" // everything is valid
	}

	var selfdestruct string
	var newContract string
	if g.mustBeSelfDestructed {
		selfdestruct = "mustBeSelfDestructed"
	} else if g.mustNotBeSelfDestructed {
		selfdestruct = "mustNotBeSelfDestructed"
	}
	if g.mustBeNewContract {
		newContract = "mustBeNewContract"
	} else if g.mustNotBeNewContract {
		newContract = "mustNotBeNewContract"
	}

	out := strings.Trim(strings.Join([]string{selfdestruct, newContract}, ", "), ", ")
	return "{" + out + "}"
}

func (g *SelfDestructedGenerator) Generate(rnd *rand.Rand) (bool, bool, error) {
	var hasSelfDestroyed bool

	if !g.mustBeSelfDestructed && !g.mustNotBeSelfDestructed {
		// random true/false
		hasSelfDestroyed = rnd.Int()%2 == 0
	} else if g.mustBeSelfDestructed && g.mustNotBeSelfDestructed {
		return false, false, ErrUnsatisfiable
	} else if g.mustBeSelfDestructed && !g.mustNotBeSelfDestructed {
		hasSelfDestroyed = true
	} else {
		hasSelfDestroyed = false
	}

	var isNewContract bool
	if !g.mustBeNewContract && !g.mustNotBeNewContract {
		// random true/false
		isNewContract = rnd.Int()%2 == 0
	} else if g.mustBeNewContract && g.mustNotBeNewContract {
		return false, false, ErrUnsatisfiable
	} else if g.mustBeNewContract && !g.mustNotBeNewContract {
		isNewContract = true
	} else {
		isNewContract = false
	}

	return hasSelfDestroyed, isNewContract, nil
}
