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
)

func TestNewInterpreter_ProducesInstanceWithSanctionedProperties(t *testing.T) {
	sfvm, err := NewInterpreter(Config{})
	if err != nil {
		t.Fatalf("failed to create SFVM instance: %v", err)
	}
	if sfvm.config.withShaCache != true {
		t.Fatalf("SFVM is not configured with sha cache")
	}
	if sfvm.config.withAnalysisCache != true {
		t.Fatalf("sfvm is not configured with analysis cache")
	}
}

func TestSfvm_OfficialConfigurationHasSanctionedProperties(t *testing.T) {
	vm, err := tosca.NewInterpreter("sfvm")
	if err != nil {
		t.Fatalf("sfvm is not registered: %v", err)
	}
	sfvm, ok := vm.(*sfvm)
	if !ok {
		t.Fatalf("unexpected interpreter implementation, got %T", vm)
	}
	if sfvm.config.withShaCache != true {
		t.Fatalf("sfvm is not configured with sha cache")
	}
	if sfvm.config.withAnalysisCache != true {
		t.Fatalf("sfvm is not configured with analysis cache")
	}
}

func TestSfvm_CachesCanBeEnabledAndDisabledInConfig(t *testing.T) {
	for _, withShaCache := range []bool{true, false} {
		for _, withAnalysisCache := range []bool{true, false} {
			config := config{
				withShaCache:      withShaCache,
				withAnalysisCache: withAnalysisCache,
			}
			vm, err := newVm(config)
			if err != nil {
				t.Fatalf("failed to create sfvm instance: %v", err)
			}
			if vm.config.withShaCache != withShaCache {
				t.Fatalf("sfvm sha cache config mismatch: expected %v, got %v",
					withShaCache, vm.config.withShaCache)
			}
			if vm.config.withAnalysisCache != withAnalysisCache {
				t.Fatalf("sfvm analysis cache config mismatch: expected %v, got %v",
					withAnalysisCache, vm.config.withAnalysisCache)
			}
		}
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
