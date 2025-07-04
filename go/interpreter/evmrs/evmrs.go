// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package evmrs

/*
#cgo LDFLAGS: -Wl,-rpath,${SRCDIR}/../../../rust/target/release
*/
import "C"

import (
	"fmt"

	"github.com/0xsoniclabs/tosca/go/interpreter/evmc"
	"github.com/0xsoniclabs/tosca/go/tosca"
)

func init() {
	// In the CGO instructions at the top of this file the build directory
	// of the evmrs project is added to the rpath of the resulting library.
	// This way, the libevmrs.so file can be found during runtime, even if
	// the LD_LIBRARY_PATH is not set accordingly.
	{
		evm, err := evmc.LoadEvmcInterpreter("libevmrs.so")
		if err != nil {
			panic(fmt.Errorf("failed to load evmrs library: %s", err))
		}
		// This instance remains in its basic configuration.
		tosca.MustRegisterInterpreterFactory("evmrs", func(any) (tosca.Interpreter, error) {
			return &evmrsInstance{evm}, nil
		})
	}

	{
		evm, err := evmc.LoadEvmcInterpreter("libevmrs.so")
		if err != nil {
			panic(fmt.Errorf("failed to load evmrs library: %s", err))
		}
		if err = evm.SetOption("logging", "true"); err != nil {
			panic(fmt.Errorf("failed to configure EVM instance: %s", err))
		}
		tosca.MustRegisterInterpreterFactory("evmrs-logging", func(any) (tosca.Interpreter, error) {
			return &evmrsInstance{evm}, nil
		})
	}
}

type evmrsInstance struct {
	e *evmc.EvmcInterpreter
}

const newestSupportedRevision = tosca.R14_Prague

func (e *evmrsInstance) Run(params tosca.Parameters) (tosca.Result, error) {
	if params.Revision > newestSupportedRevision {
		return tosca.Result{}, &tosca.ErrUnsupportedRevision{Revision: params.Revision}
	}
	return e.e.Run(params)
}
