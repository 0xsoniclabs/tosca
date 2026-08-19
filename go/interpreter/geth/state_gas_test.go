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
	"testing"

	"github.com/0xsoniclabs/tosca/go/ct/st"
	"github.com/0xsoniclabs/tosca/go/ct/utils"
	"github.com/0xsoniclabs/tosca/go/geth_adapter"
	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/0xsoniclabs/tosca/go/tosca/vm"
	geth_common "github.com/ethereum/go-ethereum/common"
	geth_vm "github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/params"
	"go.uber.org/mock/gomock"
)

// Creating a storage slot is the cheapest way to trigger a charge in the state
// dimension. Under EIP-8038 the access and the write are charged as regular
// gas, while the durable growth is charged in the state dimension.
const (
	sstoreRegularCost = tosca.Gas(params.ColdStorageAccessAmsterdam + params.StorageWriteAmsterdam)
	sstoreStateCost   = tosca.Gas(params.StorageCreationSize * params.CostPerStateByte)
	twoPushesCost     = tosca.Gas(2 * 3)
)

// TestGeth_StateGasIsChargedToTheReservoirAndReported covers the plumbing of
// the EIP-8037 state dimension across the Tosca interface: the reservoir passed
// in has to pay for state growth, the regular gas must be left alone, and the
// charge has to be reported back so a caller can reconstruct the split.
func TestGeth_StateGasIsChargedToTheReservoirAndReported(t *testing.T) {
	const gas = tosca.Gas(200_000)

	tests := map[string]struct {
		reservoir tosca.Gas
		wantGas   tosca.Gas
	}{
		"reservoir covers the charge": {
			reservoir: sstoreStateCost,
			wantGas:   gas - twoPushesCost - sstoreRegularCost,
		},
		"reservoir covers part of the charge": {
			reservoir: sstoreStateCost / 2,
			// The uncovered half falls back to regular gas.
			wantGas: gas - twoPushesCost - sstoreRegularCost - sstoreStateCost/2,
		},
		"no reservoir": {
			reservoir: 0,
			wantGas:   gas - twoPushesCost - sstoreRegularCost - sstoreStateCost,
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			result, err := runStoringInterpreter(t, gas, test.reservoir)
			if err != nil {
				t.Fatalf("failed to run: %v", err)
			}
			if !result.Success {
				t.Fatalf("execution was not successful")
			}
			if got, want := result.StateGasCharged, sstoreStateCost; got != want {
				t.Errorf("unexpected state gas charged, wanted %d, got %d", want, got)
			}
			if got, want := result.GasLeft, test.wantGas; got != want {
				t.Errorf("unexpected gas left, wanted %d, got %d", want, got)
			}

			// The caller reconstructs the split from the reservoir it provided.
			fromReservoir, fromGas := tosca.SplitStateGasCharge(test.reservoir, result.StateGasCharged)
			if got, want := fromReservoir+fromGas, result.StateGasCharged; got != want {
				t.Errorf("split does not add up, wanted %d, got %d", want, got)
			}
			if got, want := fromGas, max(0, sstoreStateCost-test.reservoir); got != want {
				t.Errorf("unexpected fallback to regular gas, wanted %d, got %d", want, got)
			}
		})
	}
}

// TestGeth_StateGasChargesCanExhaustRegularGas shows the other side of the
// fallback: without a reservoir the state charge has to fit into regular gas.
func TestGeth_StateGasChargesCanExhaustRegularGas(t *testing.T) {
	// Enough for the pushes and the regular part of the SSTORE, but not for the
	// state growth on top of it.
	gas := twoPushesCost + sstoreRegularCost + sstoreStateCost - 1

	result, err := runStoringInterpreter(t, gas, 0)
	if err != nil {
		t.Fatalf("failed to run: %v", err)
	}
	if result.Success {
		t.Error("execution should have run out of gas")
	}

	// With a reservoir covering the state dimension the very same budget works.
	result, err = runStoringInterpreter(t, gas, sstoreStateCost)
	if err != nil {
		t.Fatalf("failed to run: %v", err)
	}
	if !result.Success {
		t.Error("execution should have succeeded")
	}
}

// runStoringInterpreter executes an SSTORE creating a fresh storage slot at
// Amsterdam.
func runStoringInterpreter(t *testing.T, gas, reservoir tosca.Gas) (tosca.Result, error) {
	t.Helper()

	code := []byte{
		byte(vm.PUSH1), 1, // < the value to be stored
		byte(vm.PUSH1), 0, // < the slot to store it in
		byte(vm.SSTORE),
	}

	state := st.NewState(st.NewCode(code))
	defer state.Release()
	state.Revision = tosca.R16_Amsterdam

	parameters := utils.ToVmParameters(state)
	parameters.Gas = gas
	parameters.StateGas = reservoir

	return (&gethVm{}).Run(parameters)
}

// TestCtAdapter_InterceptedCallsCarryTheStateDimension covers the third place
// the state dimension crosses the interface: a call made from within geth is
// routed out to Tosca, so the reservoir has to be forwarded to the callee and
// the charge it reports has to be turned back into the leftover budget the
// calling geth frame absorbs.
func TestCtAdapter_InterceptedCallsCarryTheStateDimension(t *testing.T) {
	const (
		gas       = 1000
		reservoir = 500
	)

	tests := map[string]struct {
		charged     tosca.Gas
		wantState   uint64
		wantSpilled uint64
	}{
		"nothing charged":   {charged: 0, wantState: reservoir, wantSpilled: 0},
		"partial charge":    {charged: 200, wantState: 300, wantSpilled: 0},
		"reservoir used up": {charged: reservoir, wantState: 0, wantSpilled: 0},
		"borrowing gas":     {charged: 700, wantState: 0, wantSpilled: 200},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			context := tosca.NewMockRunContext(ctrl)

			context.EXPECT().Call(tosca.DelegateCall, gomock.Any()).DoAndReturn(
				func(_ tosca.CallKind, parameter tosca.CallParameters) (tosca.CallResult, error) {
					if got, want := parameter.StateGas, tosca.Gas(reservoir); got != want {
						t.Errorf("unexpected reservoir forwarded, wanted %d, got %d", want, got)
					}
					return tosca.CallResult{
						Success:         true,
						GasLeft:         parameter.Gas,
						StateGasCharged: test.charged,
					}, nil
				})

			interceptor := &callInterceptor{
				parameters: tosca.Parameters{Context: context},
				stateDb:    geth_adapter.NewStateDB(context),
			}

			entry := geth_vm.NewGasBudget(gas, reservoir)
			_, left, err := interceptor.DelegateCall(nil, geth_common.Address{}, geth_common.Address{}, nil, entry)
			if err != nil {
				t.Fatalf("unexpected error: %v", err)
			}

			if got, want := left.RegularGas, uint64(gas); got != want {
				t.Errorf("unexpected regular gas left, wanted %d, got %d", want, got)
			}
			if got, want := left.StateGas, test.wantState; got != want {
				t.Errorf("unexpected reservoir left, wanted %d, got %d", want, got)
			}
			if got, want := left.Spilled, test.wantSpilled; got != want {
				t.Errorf("unexpected borrowed regular gas, wanted %d, got %d", want, got)
			}
			if got, want := left.UsedStateGas, int64(test.charged); got != want {
				t.Errorf("unexpected state gas used, wanted %d, got %d", want, got)
			}
		})
	}
}

// TestGeth_RevertRefundsStateGasAndTheGasBorrowedForIt covers the exit form of
// the budget: EIP-8037 refunds the state gas a reverting frame charged, so the
// regular gas that had to be borrowed for it comes back to the caller too.
func TestGeth_RevertRefundsStateGasAndTheGasBorrowedForIt(t *testing.T) {
	const gas = tosca.Gas(200_000)

	// Store into a fresh slot and then revert. Without a reservoir the state
	// growth is paid from regular gas, and the revert has to return it.
	code := []byte{
		byte(vm.PUSH1), 1, // < the value to be stored
		byte(vm.PUSH1), 0, // < the slot to store it in
		byte(vm.SSTORE),
		byte(vm.PUSH1), 0,
		byte(vm.PUSH1), 0,
		byte(vm.REVERT),
	}
	const revertCost = 4 * 3 // < the two pushes of the store and the two of the revert

	state := st.NewState(st.NewCode(code))
	defer state.Release()
	state.Revision = tosca.R16_Amsterdam

	parameters := utils.ToVmParameters(state)
	parameters.Gas = gas

	result, err := (&gethVm{}).Run(parameters)
	if err != nil {
		t.Fatalf("failed to run: %v", err)
	}
	if result.Success {
		t.Fatal("execution should have reverted")
	}

	// The state charge is refunded, so only the regular costs remain deducted.
	if got, want := result.StateGasCharged, tosca.Gas(0); got != want {
		t.Errorf("unexpected state gas charged, wanted %d, got %d", want, got)
	}
	if got, want := result.GasLeft, gas-revertCost-sstoreRegularCost; got != want {
		t.Errorf("unexpected gas left, wanted %d, got %d", want, got)
	}
}
