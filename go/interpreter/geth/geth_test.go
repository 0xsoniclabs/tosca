// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package geth

import (
	"math/big"
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/ethereum/go-ethereum/params"
)

func TestGethInterpreter_MakeChainConfigSetsTheCorrectRevision(t *testing.T) {
	revisions := []tosca.Revision{
		tosca.R07_Istanbul,
		tosca.R09_Berlin,
		tosca.R10_London,
		tosca.R11_Paris,
		tosca.R12_Shanghai,
		tosca.R13_Cancun,
		tosca.R14_Prague,
	}

	for _, targetRevision := range revisions {
		t.Run(targetRevision.String(), func(t *testing.T) {
			chainConfig := MakeChainConfig(
				*params.AllEthashProtocolChanges,
				big.NewInt(0),
				targetRevision,
			)

			revisionChecksPreParis := map[tosca.Revision]func(blockNumber *big.Int) bool{
				tosca.R07_Istanbul: chainConfig.IsIstanbul,
				tosca.R09_Berlin:   chainConfig.IsBerlin,
				tosca.R10_London:   chainConfig.IsLondon,
			}
			revisionChecksPostParis := map[tosca.Revision]func(blockNumber *big.Int, blockTime uint64) bool{
				tosca.R12_Shanghai: chainConfig.IsShanghai,
				tosca.R13_Cancun:   chainConfig.IsCancun,
				tosca.R14_Prague:   chainConfig.IsPrague,
			}

			blockNumber := big.NewInt(0)
			blockTime := uint64(0)

			for revision, check := range revisionChecksPreParis {
				if revision <= targetRevision && !check(blockNumber) {
					t.Errorf("%s is before %s and should be supported", revision, targetRevision)
				}
				if revision > targetRevision && check(blockNumber) {
					t.Errorf("%s is after %s and should not be supported", revision, targetRevision)
				}
			}
			for revision, check := range revisionChecksPostParis {
				if revision <= targetRevision && !check(blockNumber, blockTime) {
					t.Errorf("%s is before %s and should be supported", revision, targetRevision)
				}
				if revision > targetRevision && check(blockNumber, blockTime) {
					t.Errorf("%s is after %s and should not be supported", revision, targetRevision)
				}
			}
		})
	}

}
