// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package floria_eth

import (
	"github.com/0xsoniclabs/tosca/go/processor/floria"
	"github.com/0xsoniclabs/tosca/go/tosca"
)

func init() {
	// Register an ethereum compatible version of the geth processor.
	tosca.RegisterProcessorFactory("floria-eth", newFloriaEthProcessor)
}

// newFloriaProcessor creates a new instance of the Floria processor with the given interpreter.
// This version of Floria is compatible with the Ethereum blockchain, but does not support
// Sonic.
func newFloriaEthProcessor(interpreter tosca.Interpreter) tosca.Processor {
	return &floria.Processor{
		Interpreter:   interpreter,
		EthCompatible: true,
	}
}
