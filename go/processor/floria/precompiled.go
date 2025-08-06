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
	"fmt"
	"math"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/ethereum/go-ethereum/common"
	geth "github.com/ethereum/go-ethereum/core/vm"
)

func isPrecompiled(address tosca.Address, revision tosca.Revision) bool {
	_, ok := getPrecompiledContract(address, revision)
	return ok
}

func runPrecompiledContract(revision tosca.Revision, input tosca.Data, address tosca.Address, gas tosca.Gas) (tosca.CallResult, error) {
	contract, ok := getPrecompiledContract(address, revision)
	if !ok {
		return tosca.CallResult{}, fmt.Errorf("precompiled contract not found")
	}
	gasCost := contract.RequiredGas(input)
	if gasCost > math.MaxInt64 {
		return tosca.CallResult{}, fmt.Errorf("gas cost exceeds maximum limit")
	}
	if gas < tosca.Gas(gasCost) {
		return tosca.CallResult{}, fmt.Errorf("insufficient gas")
	}
	gas -= tosca.Gas(gasCost)
	output, err := contract.Run(input)
	if err != nil {
		return tosca.CallResult{}, fmt.Errorf("error executing precompiled contract: %w", err)
	}

	return tosca.CallResult{
		Success: true,
		Output:  output,
		GasLeft: gas,
	}, nil
}

func getPrecompiledContract(address tosca.Address, revision tosca.Revision) (geth.PrecompiledContract, bool) {
	precompiles := getPrecompiledContracts(revision)
	contract, ok := precompiles[common.Address(address)]
	return contract, ok
}

func getPrecompiledContracts(revision tosca.Revision) map[common.Address]geth.PrecompiledContract {
	var precompiles map[common.Address]geth.PrecompiledContract
	switch revision {
	case tosca.R13_Cancun:
		precompiles = geth.PrecompiledContractsCancun
	case tosca.R12_Shanghai, tosca.R11_Paris, tosca.R10_London, tosca.R09_Berlin:
		precompiles = geth.PrecompiledContractsBerlin
	default: // Istanbul is the oldest supported revision supported by Sonic
		precompiles = geth.PrecompiledContractsIstanbul
	}
	return precompiles
}

func getPrecompiledAddresses(revision tosca.Revision) []tosca.Address {
	precompiles := getPrecompiledContracts(revision)
	addresses := make([]tosca.Address, 0, len(precompiles))
	for addr := range precompiles {
		addresses = append(addresses, tosca.Address(addr))
	}
	return addresses
}
