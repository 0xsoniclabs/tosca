// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package evmc

import (
	"math"
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/ethereum/evmc/v11/bindings/go/evmc"
	"github.com/stretchr/testify/require"
	"go.uber.org/mock/gomock"
)

func TestEvmcInterpreter_RunDerivesHostContextFromParameters(t *testing.T) {
	interpreter := newTestInterpreter()
	tests := map[string][]tosca.Hash{
		"none":     nil,
		"one":      {{1}},
		"several":  {{2}, {3}, {4}},
		"repeated": {{5}, {5}},
	}

	for name, hashes := range tests {
		t.Run(name, func(t *testing.T) {
			runContext := tosca.NewMockRunContext(gomock.NewController(t))
			// The test VM asks whether the recipient exists; a non-zero nonce
			// ends isEmpty at its first condition, so the balance and the code
			// size are never asked for and need no expectation of their own.
			runContext.EXPECT().GetNonce(tosca.Address{}).Return(uint64(1))

			result, err := interpreter.Run(tosca.Parameters{
				Context:               runContext,
				TransactionParameters: tosca.TransactionParameters{BlobHashes: hashes},
			})

			require.NoError(t, err)
			// The test VM returns the blob hashes of its host context as output.
			want := []byte{}
			for _, hash := range hashes {
				want = append(want, hash[:]...)
			}
			require.Equal(t, want, []byte(result.Output))
		})
	}
}

func TestEvmcInterpreter_RevisionConversion(t *testing.T) {
	tests := []struct {
		tosca tosca.Revision
		evmc  evmc.Revision
	}{
		{tosca.R07_Istanbul, evmc.Istanbul},
		{tosca.R09_Berlin, evmc.Berlin},
		{tosca.R10_London, evmc.London},
		{tosca.R11_Paris, evmc.Paris},
		{tosca.R12_Shanghai, evmc.Shanghai},
	}

	for _, test := range tests {
		want := test.evmc
		got, err := toEvmcRevision(test.tosca)
		if err != nil {
			t.Fatalf("unexpected error during conversion: %v", err)
		}
		if want != got {
			t.Errorf("unexpected conversion of %v, wanted %v, got %v", test.tosca, want, got)
		}
	}
}

func TestEvmcInterpreter_RevisionConversionFailsOnUnknownRevision(t *testing.T) {
	_, err := toEvmcRevision(tosca.Revision(math.MaxInt))
	if err == nil {
		t.Errorf("expected a conversion failure, got nothing")
	}
}
