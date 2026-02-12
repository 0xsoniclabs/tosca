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
	"fmt"
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/stretchr/testify/require"
)

func TestSfvm_OfficialConfigurationHasSanctionedProperties(t *testing.T) {
	vm, err := tosca.NewInterpreter("sfvm")
	if err != nil {
		t.Fatalf("sfvm is not registered: %v", err)
	}
	sfvm, ok := vm.(*sfvm)
	if !ok {
		t.Fatalf("unexpected interpreter implementation, got %T", vm)
	}
	if !sfvm.config.WithShaCache {
		t.Fatalf("sfvm is not configured with sha cache")
	}
	if !sfvm.config.WithAnalysisCache {
		t.Fatalf("sfvm is not configured with analysis cache")
	}
	if sfvm.analysis.maxCachedCodeSize != 1<<14+1<<13 {
		t.Fatalf("sfvm analysis cache max cached code size mismatch: expected %d, got %d",
			1<<14+1<<13, sfvm.analysis.maxCachedCodeSize)
	}
}

func TestSfvm_CachesCanBeEnabledAndDisabledInConfig(t *testing.T) {
	for _, withShaCache := range []bool{true, false} {
		for _, withAnalysisCache := range []bool{true, false} {
			config := Config{
				WithShaCache:      withShaCache,
				WithAnalysisCache: withAnalysisCache,
			}
			vm, err := NewInterpreter(config)
			if err != nil {
				t.Fatalf("failed to create sfvm instance: %v", err)
			}
			if vm.config.WithShaCache != withShaCache {
				t.Fatalf("sfvm sha cache config mismatch: expected %v, got %v",
					withShaCache, vm.config.WithShaCache)
			}
			if vm.config.WithAnalysisCache != withAnalysisCache {
				t.Fatalf("sfvm analysis cache config mismatch: expected %v, got %v",
					withAnalysisCache, vm.config.WithAnalysisCache)
			}
			require.Equal(t, withAnalysisCache, vm.analysis.cache != nil,
				"sfvm analysis cache presence mismatch",
			)
		}
	}
}

func TestNewInterpreter_AnalysisCacheArgumentsAreForwarded(t *testing.T) {
	maxCachedCodeSize := 42
	cacheSize := 42424242
	vm, err := NewInterpreter(Config{
		WithAnalysisCache: true,
		AnalysisCacheSize: cacheSize,
		MaxCachedCodeSize: maxCachedCodeSize,
	})
	if err != nil {
		t.Fatalf("failed to create sfvm instance: %v", err)
	}

	if !vm.config.WithAnalysisCache {
		t.Fatalf("config value has not been forwarded correctly expected true, got %v",
			vm.config.WithAnalysisCache)
	}
	if vm.analysis.maxCachedCodeSize != maxCachedCodeSize {
		t.Fatalf("unexpected maxCachedCodeSize: expected %d, got %d",
			maxCachedCodeSize, vm.analysis.maxCachedCodeSize)
	}
}

func TestSfvm_InterpreterReturnsErrorWhenExecutingUnsupportedRevision(t *testing.T) {
	vm, err := tosca.NewInterpreter("sfvm")
	if err != nil {
		t.Fatalf("sfvm is not registered: %v", err)
	}

	params := tosca.Parameters{}
	params.Revision = newestSupportedRevision + 1

	_, err = vm.Run(params)
	if want, got := fmt.Sprintf("unsupported revision %d", params.Revision), err.Error(); want != got {
		t.Fatalf("unexpected error: want %q, got %q", want, got)
	}
}
