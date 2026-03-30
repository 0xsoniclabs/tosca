// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package main

import (
	"fmt"
	"sort"

	cliUtils "github.com/0xsoniclabs/tosca/go/ct/driver/cli"
	"github.com/0xsoniclabs/tosca/go/ct/smt"
	"github.com/0xsoniclabs/tosca/go/ct/smt/cvc5"
	"github.com/0xsoniclabs/tosca/go/ct/spc"
	"github.com/urfave/cli/v2"
)

var SmtCheckerCmd = cli.Command{
	Action: doSmtChecker,
	Name:   "smt-checker",
	Usage:  "Checks specification determinism and completeness using CVC5 SMT solver",
	Flags: []cli.Flag{
		cliUtils.FilterFlag,
		&cli.StringFlag{
			Name:  "check",
			Usage: "type of check to run: determinism, completeness, or both",
			Value: "both",
		},
	},
}

func doSmtChecker(context *cli.Context) error {
	filter, err := cliUtils.FilterFlag.Fetch(context)
	if err != nil {
		return err
	}

	checkType := context.String("check")
	runDeterminism := checkType == "determinism" || checkType == "both"
	runCompleteness := checkType == "completeness" || checkType == "both"

	// Get and filter rules.
	rules := spc.FilterRules(spc.Spec.GetRules(), filter)
	sort.Slice(rules, func(i, j int) bool { return rules[i].Name < rules[j].Name })

	fmt.Printf("Read specification ... %d rules\n", len(rules))

	// Create CVC5 context.
	ctx := cvc5.NewContext()

	// Convert rlz.Rule to smt.Rule by capturing the Cvc method.
	smtRules := make([]smt.Rule, len(rules))
	for i, rule := range rules {
		rule := rule // capture loop variable
		smtRules[i] = smt.Rule{
			Name: rule.Name,
			Condition: func(ctx smt.Context) smt.Term {
				return rule.Condition.Cvc(ctx)
			},
			Effect: rule.Effect.String(),
		}
	}

	allPassed := true

	if runDeterminism {
		fmt.Println("Check determinism ...")
		if smt.CheckDeterminism(smtRules, ctx) {
			fmt.Println("\tSpecification is deterministic.")
		} else {
			fmt.Println("\tSpecification is not deterministic.")
			allPassed = false
		}
	}

	if runCompleteness {
		fmt.Println("\nCheck completeness ...")
		if smt.CheckCompleteness(smtRules, ctx) {
			fmt.Println("\tSpecification is complete.")
		} else {
			fmt.Println("\tSpecification is not complete.")
			allPassed = false
		}
	}

	if !allPassed {
		return fmt.Errorf("specification checks failed")
	}
	return nil
}
