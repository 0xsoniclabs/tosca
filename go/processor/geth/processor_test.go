// Copyright (c) 2024 Fantom Foundation
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at fantom.foundation/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package geth_processor

import (
	"math/big"
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"go.uber.org/mock/gomock"
)

func TestGethProcessor_RevisionConversion(t *testing.T) {
	tests := map[string]struct {
		blockNumber int64
		timestamp   int64
		revision    tosca.Revision
	}{
		"Istanbul": {
			blockNumber: 1000,
			timestamp:   0,
			revision:    tosca.R07_Istanbul,
		},
		"Berlin": {
			blockNumber: 2000,
			timestamp:   0,
			revision:    tosca.R09_Berlin,
		},
		"London": {
			blockNumber: 3000,
			timestamp:   0,
			revision:    tosca.R10_London,
		},
		"Merge": {
			blockNumber: 4000,
			timestamp:   0,
			revision:    tosca.R11_Paris,
		},
		"Shanghai": {
			blockNumber: 5000,
			timestamp:   100,
			revision:    tosca.R12_Shanghai,
		},
		"Cancun": {
			blockNumber: 6000,
			timestamp:   200,
			revision:    tosca.R13_Cancun,
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			toscaBlockParameters := tosca.BlockParameters{
				BlockNumber: test.blockNumber,
				Timestamp:   test.timestamp,
				Revision:    test.revision,
			}

			chainConfig := blockParametersToChainConfig(toscaBlockParameters)
			rules := chainConfig.Rules(big.NewInt(test.blockNumber), test.revision >= tosca.R11_Paris, uint64(test.timestamp))

			if ((test.revision >= tosca.R13_Cancun) != rules.IsCancun) ||
				((test.revision >= tosca.R12_Shanghai) != rules.IsShanghai) ||
				((test.revision >= tosca.R11_Paris) != rules.IsMerge) ||
				((test.revision >= tosca.R10_London) != rules.IsLondon) ||
				((test.revision >= tosca.R09_Berlin) != rules.IsBerlin) ||
				((test.revision >= tosca.R07_Istanbul) != rules.IsIstanbul) {
				t.Errorf("revision %v is not converted correctly, %v %v", test.revision, (test.revision >= tosca.R10_London), rules.IsLondon)
			}
		})
	}
}

func TestGethProcessor_ConfigAddsStateContract(t *testing.T) {
	ctrl := gomock.NewController(t)
	interpreter := tosca.NewMockInterpreter(ctrl)
	config := newEVMConfig(interpreter, false)
	_, ok := config.StatePrecompiles[stateContractAddress]
	if !ok {
		t.Errorf("state contract not added to config")
	}
}
