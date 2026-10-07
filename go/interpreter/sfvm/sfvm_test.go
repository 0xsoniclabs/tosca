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
	"bytes"
	"fmt"
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/0xsoniclabs/tosca/go/tosca/vm"
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

func TestNewInterpreter_NonPositiveConfigValuesUseDefaults(t *testing.T) {
	tests := map[string]Config{
		"zero values":     {WithAnalysisCache: true},
		"negative values": {WithAnalysisCache: true, AnalysisCacheSize: -1, MaxCachedCodeSize: -1},
	}

	for name, config := range tests {
		t.Run(name, func(t *testing.T) {
			require := require.New(t)
			vm, err := NewInterpreter(config)
			require.NoError(err)

			require.Equal(1<<14+1<<13, vm.analysis.maxCachedCodeSize)
			require.NotNil(vm.analysis.cache)
		})
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

func TestSfvm_Run_AcceptsNewestSupportedRevision(t *testing.T) {
	require := require.New(t)
	interpreter, err := NewInterpreter(Config{})
	require.NoError(err)

	result, err := interpreter.Run(tosca.Parameters{
		BlockParameters: tosca.BlockParameters{Revision: newestSupportedRevision},
		Gas:             10,
		Code:            tosca.Code{byte(vm.STOP)},
	})

	require.NoError(err)
	require.Equal(tosca.Result{Success: true, GasLeft: 10}, result)
}

func TestSfvm_Run_ForwardsShaCacheConfiguration(t *testing.T) {
	// The SHA3 cache is a package-level instance; whether a 32-byte input is
	// cached after a run is the only observable effect of the flag.
	tests := map[string]struct {
		withShaCache bool
		marker       byte
		cached       bool
	}{
		"cache disabled": {withShaCache: false, marker: 0xd1, cached: false},
		"cache enabled":  {withShaCache: true, marker: 0xd2, cached: true},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			require := require.New(t)
			interpreter, err := NewInterpreter(Config{WithShaCache: test.withShaCache})
			require.NoError(err)
			input := bytes.Repeat([]byte{test.marker}, 32)
			code := append(tosca.Code{byte(vm.PUSH32)}, input...)
			code = append(code,
				byte(vm.PUSH1), 0, byte(vm.MSTORE),
				byte(vm.PUSH1), 32, byte(vm.PUSH1), 0, byte(vm.SHA3),
				byte(vm.STOP),
			)

			result, err := interpreter.Run(tosca.Parameters{Gas: 1_000, Code: code})
			require.NoError(err)
			require.True(result.Success)

			sha3Cache.cache32.lock.Lock()
			_, cached := sha3Cache.cache32.index[[32]byte(input)]
			sha3Cache.cache32.lock.Unlock()
			require.Equal(test.cached, cached)
		})
	}
}
