// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package smt

import "fmt"

// Rule is a minimal representation of a specification rule for the prover.
// It avoids importing the rlz package to prevent circular dependencies.
type Rule struct {
	Name      string
	Condition func(ctx Context) Term // produces the SMT condition term
	Effect    string                 // effect name for determinism comparison
}

// CheckDeterminism verifies that no two rules with different effects
// have overlapping conditions. Returns true if the specification is
// deterministic.
func CheckDeterminism(rules []Rule, ctx Context) bool {
	deterministic := true
	for i := range rules {
		for j := range i {
			if rules[i].Effect != rules[j].Effect {
				condI := rules[i].Condition(ctx)
				condJ := rules[j].Condition(ctx)
				overlap := ctx.And(condI, condJ)
				sat, conclusive := ctx.CheckSatWith(overlap)
				if conclusive && sat {
					fmt.Printf("=> Check rules %s and %s\n", rules[i].Name, rules[j].Name)
					fmt.Printf("\tRules overlap and make specification indeterministic.\n")
					deterministic = false
				}
			}
		}
	}
	return deterministic
}

// CheckCompleteness verifies that every valid VM state is covered by
// at least one rule condition. Returns true if the specification is
// complete.
func CheckCompleteness(rules []Rule, ctx Context) bool {
	if len(rules) == 0 {
		fmt.Printf("\tNo rules to check.\n")
		return false
	}

	// Build disjunction of all rule conditions.
	conditions := make([]Term, len(rules))
	for i, rule := range rules {
		conditions[i] = rule.Condition(ctx)
	}
	cover := ctx.Or(conditions...)

	// Check if there exists a state where no rule applies.
	sat, conclusive := ctx.CheckSatWith(ctx.Not(cover))
	if conclusive && sat {
		fmt.Printf("\tFound uncovered state.\n")
		return false
	}
	return true
}
