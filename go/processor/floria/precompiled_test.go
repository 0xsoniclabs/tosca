// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package floria

import (
	"strings"
	"testing"

	test_utils "github.com/0xsoniclabs/tosca/go/processor"
	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/stretchr/testify/require"
)

func TestPrecompiled_RightNumberOfContractsDependingOnRevision(t *testing.T) {
	tests := []struct {
		revision          tosca.Revision
		numberOfContracts int
	}{
		{tosca.R07_Istanbul, 9},
		{tosca.R09_Berlin, 9},
		{tosca.R10_London, 9},
		{tosca.R11_Paris, 9},
		{tosca.R12_Shanghai, 9},
		{tosca.R13_Cancun, 10},
	}

	for _, test := range tests {
		count := 0
		for i := byte(0x01); i < byte(0x42); i++ {
			address := test_utils.NewAddress(i)
			isPrecompiled := isPrecompiled(address, test.revision)
			if isPrecompiled {
				count++
			}
		}
		if count != test.numberOfContracts {
			t.Errorf("unexpected number of precompiled contracts for revision %v, want %v, got %v", test.revision, test.numberOfContracts, count)
		}
		if len(getPrecompiledAddresses(test.revision)) != test.numberOfContracts {
			t.Errorf("unexpected number of precompiled contracts for revision %v, want %v, got %v", test.revision, test.numberOfContracts, count)
		}
	}
}

func TestPrecompiled_AddressesAreHandledCorrectly(t *testing.T) {
	tests := map[string]struct {
		revision      tosca.Revision
		address       tosca.Address
		gas           tosca.Gas
		isPrecompiled bool
		success       bool
	}{
		"nonPrecompiled":            {tosca.R09_Berlin, test_utils.NewAddress(0x20), 3000, false, false},
		"ecrecover-success":         {tosca.R10_London, test_utils.NewAddress(0x01), 3000, true, true},
		"ecrecover-outOfGas":        {tosca.R10_London, test_utils.NewAddress(0x01), 1, true, false},
		"pointEvaluation-success":   {tosca.R13_Cancun, test_utils.NewAddress(0x0a), 55000, true, true},
		"pointEvaluation-outOfGas":  {tosca.R13_Cancun, test_utils.NewAddress(0x0a), 1, true, false},
		"pointEvaluation-preCancun": {tosca.R10_London, test_utils.NewAddress(0x0a), 3000, false, false},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {

			input := tosca.Data{}
			if strings.Contains(name, "pointEvaluation") {
				input = test_utils.ValidPointEvaluationInput
			}

			isPrecompiled := isPrecompiled(test.address, test.revision)
			if isPrecompiled != test.isPrecompiled {
				t.Fatalf("unexpected precompiled, want %v, got %v", test.isPrecompiled, isPrecompiled)
			}

			result, err := runPrecompiledContract(test.revision, input, test.address, test.gas)
			if test.success {
				require.NoError(t, err)
				require.True(t, result.Success)
			} else {
				require.Error(t, err)
			}
		})
	}
}

func TestPrecompiled_ErrorsAreHandledCorrectly(t *testing.T) {
	tests := map[string]struct {
		address       tosca.Address
		gas           tosca.Gas
		expectedError string
	}{
		"nonPrecompiled": {
			tosca.Address{0x20},
			3000,
			"precompiled contract not found",
		},
		"ecrecover-insufficientGas": {
			test_utils.NewAddress(0x01), // ecrecover address
			1,
			"insufficient gas",
		},
		"failing-input": {
			test_utils.NewAddress(0x0a), // point evaluation address
			55000,
			"error executing precompiled contract",
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			input := tosca.Data{}
			result, err := runPrecompiledContract(tosca.R13_Cancun, input, test.address, test.gas)
			require.ErrorContains(t, err, test.expectedError)
			require.False(t, result.Success, "expected the result to be unsuccessful due to error")
			require.Equal(t, tosca.Gas(0), result.GasLeft, "expected gas left to be zero on error")
		})
	}
}

func TestPrecompiled_GasCostOverflowIsDetectedAndHandled(t *testing.T) {
	// Input data to produce a gas price which overflows int64 in the MODEXP precompiled contract.
	data := []byte{0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
		0, 0, 0, 0, 0, 32, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
		255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
		255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
		255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 254, 255, 255, 255, 255, 255, 255, 255, 255,
		255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
		255, 255, 255, 255, 253,
	}

	modExpAddress := test_utils.NewAddress(0x05)
	result, err := runPrecompiledContract(tosca.R13_Cancun, tosca.Data(data), modExpAddress, 100)
	require.ErrorContains(t, err, "gas cost exceeds maximum limit")
	require.False(t, result.Success, "expected the result to be unsuccessful due to gas cost overflow")
}
