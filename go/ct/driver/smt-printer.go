// Copyright (c) 2025 Sonic Operations Ltd
//
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
	"github.com/0xsoniclabs/tosca/go/ct/spc"
	"github.com/urfave/cli/v2"
)

var SmtPrinterCmd = cli.Command{
	Action: doSmtPrinter,
	Name:   "smt-printer",
	Usage:  "Produces Python code for checking rule consistency using an SMT solver",
	Flags: []cli.Flag{
		cliUtils.FilterFlag,
	},
}

func doSmtPrinter(context *cli.Context) error {
	filter, err := cliUtils.FilterFlag.Fetch(context)
	if err != nil {
		return err
	}

	fmt.Printf("[")
	rules := spc.FilterRules(spc.Spec.GetRules(), filter)
	sort.Slice(rules, func(i, j int) bool { return rules[i].Name < rules[j].Name })
	for i, rule := range rules {
		if i > 0 {
			fmt.Printf(", ")
		}
		fmt.Printf("(\"%v\", %v, \"%v\")", rule.Name, rule.Condition.Py(), rule.Effect)
	}
	fmt.Printf("]\n")
	return nil
}
