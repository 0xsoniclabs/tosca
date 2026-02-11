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
	WithShaCache      bool // Whether to enable caching of SHA3 computations
	WithAnalysisCache bool // Whether to enable caching of jump destination analyses
	AnalysisCacheSize int  // Maximum size of the analysis cache in bytes (default: 256 MB)
	MaxCachedCodeSize int  // Maximum code size in bytes for which analyses are cached (default: 24 KB)
}

// NewInterpreter creates a new SFVM interpreter instance with the given configuration.
func NewInterpreter(config Config) (*sfvm, error) {
	var analysis analysis
	if config.WithAnalysisCache {

		maxCachedCodeSize := 1<<14 + 1<<13 // 24 KB
		if config.MaxCachedCodeSize > 0 {
			maxCachedCodeSize = config.MaxCachedCodeSize
		}

		analysisCacheSize := 1 << 28 // 256 MB
		if config.AnalysisCacheSize > 0 {
			analysisCacheSize = config.AnalysisCacheSize
		}

		analysis = newAnalysis(analysisCacheSize, maxCachedCodeSize)
	}

	sfvm := &sfvm{
		config:   config,
		analysis: analysis,
	}
	return sfvm, nil
}

// Registers the simple form EVM as a possible interpreter implementation.
func init() {
	tosca.MustRegisterInterpreterFactory("sfvm", func(any) (tosca.Interpreter, error) {
		return NewInterpreter(Config{
			WithShaCache:      true,
			WithAnalysisCache: true,
		})
	})
}

type sfvm struct {
	config   Config
	analysis analysis
}

// Defines the newest supported revision for this interpreter implementation
const newestSupportedRevision = tosca.R15_Osaka

func (s *sfvm) Run(params tosca.Parameters) (tosca.Result, error) {
	if params.Revision > newestSupportedRevision {
		return tosca.Result{}, &tosca.ErrUnsupportedRevision{Revision: params.Revision}
	}

	return run(s.analysis, s.config, params)
}
