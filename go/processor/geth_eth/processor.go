// Copyright (c) 2024 Fantom Foundation
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at fantom.foundation/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package geth_processor_eth

import (
	geth_processor "github.com/0xsoniclabs/tosca/go/processor/geth"
	"github.com/0xsoniclabs/tosca/go/tosca"
)

func init() {
	// Register an ethereum compatible version of the geth processor.
	tosca.RegisterProcessorFactory("geth-eth", ethereumProcessor)
}

func ethereumProcessor(interpreter tosca.Interpreter) tosca.Processor {
	return &geth_processor.Processor{
		Interpreter:        interpreter,
		EthereumCompatible: true,
	}
}
