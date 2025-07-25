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
	"os"

	"github.com/urfave/cli/v2"
)

func main() {
	app := &cli.App{
		Name:      "driver",
		Usage:     "Tosca Conformance Tests Driver",
		Copyright: "(c) 2023 Fantom Foundation",
		Flags:     []cli.Flag{},
		Commands: []*cli.Command{
			&GeneratorInfoCmd,
			&ListCmd,
			&ProbeCmd,
			&RegressionsCmd,
			&RunCmd,
			&StatsCmd,
			&TestCmd,
		},
	}

	if err := app.Run(os.Args); err != nil {
		fmt.Fprintln(os.Stderr, err)
		os.Exit(1)
	}
}
