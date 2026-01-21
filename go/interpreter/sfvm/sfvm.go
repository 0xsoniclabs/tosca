// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package sfvm

import (
	"github.com/0xsoniclabs/tosca/go/tosca"
)

// Config provides a set of user-definable options for the SFVM interpreter.
type Config struct {
}

// NewInterpreter creates a new SFVM interpreter instance with the official
// configuration for production purposes.
func NewInterpreter(Config) (*sfvm, error) {
	return newVm(config{
		withShaCache:      true,
		withAnalysisCache: true,
	})
}

// Registers the long-form EVM as a possible interpreter implementation.
func init() {
	tosca.MustRegisterInterpreterFactory("sfvm", func(any) (tosca.Interpreter, error) {
		return NewInterpreter(Config{})
	})
}

type config struct {
	withShaCache      bool
	withAnalysisCache bool
}

type sfvm struct {
	config   config
	analysis analysis
}

func newVm(config config) (*sfvm, error) {
	var analysis analysis
	if config.withAnalysisCache {
		analysis = newAnalysis(1 << 30) // = 1GiB
	}

	sfvm := &sfvm{
		config:   config,
		analysis: analysis,
	}
	return sfvm, nil
}

// Defines the newest supported revision for this interpreter implementation
const newestSupportedRevision = tosca.R15_Osaka

func (s *sfvm) Run(params tosca.Parameters) (tosca.Result, error) {
	if params.Revision > newestSupportedRevision {
		return tosca.Result{}, &tosca.ErrUnsupportedRevision{Revision: params.Revision}
	}

	return run(s.analysis, s.config, params)
}
