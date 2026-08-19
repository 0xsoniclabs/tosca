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
	"testing"

	"github.com/0xsoniclabs/tosca/go/ct/st"
	"github.com/0xsoniclabs/tosca/go/ct/utils"
	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/0xsoniclabs/tosca/go/tosca/vm"
	"github.com/holiman/uint256"
	"go.uber.org/mock/gomock"
)

// Creating a storage slot is the cheapest way to trigger a charge in the state
// dimension. Under EIP-8038 the access and the write are charged as regular
// gas, while the durable growth is charged in the state dimension.
const (
	sstoreRegularCost = ColdStorageAccessCostAmsterdam + StorageWriteCostAmsterdam
	sstoreStateCost   = StorageCreationStateCostAmsterdam
	twoPushesCost     = tosca.Gas(2 * 3)
)

// TestStateGas_IsChargedToTheReservoirAndReported covers the plumbing of the
// EIP-8037 state dimension across the Tosca interface: the reservoir passed in
// has to pay for state growth, the regular gas must be left alone, and the
// charge has to be reported back so a caller can reconstruct the split.
func TestStateGas_IsChargedToTheReservoirAndReported(t *testing.T) {
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
			result, err := runStoringInterpreter(t, gas, test.reservoir, false)
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
		})
	}
}

// TestStateGas_ChargesCanExhaustRegularGas shows the other side of the fallback:
// without a reservoir the state charge has to fit into regular gas.
func TestStateGas_ChargesCanExhaustRegularGas(t *testing.T) {
	// Enough for the pushes and the regular part of the SSTORE, but not for the
	// state growth on top of it.
	gas := twoPushesCost + sstoreRegularCost + sstoreStateCost - 1

	result, err := runStoringInterpreter(t, gas, 0, false)
	if err != nil {
		t.Fatalf("failed to run: %v", err)
	}
	if result.Success {
		t.Error("execution should have run out of gas")
	}

	// With a reservoir covering the state dimension the very same budget works.
	result, err = runStoringInterpreter(t, gas, sstoreStateCost, false)
	if err != nil {
		t.Fatalf("failed to run: %v", err)
	}
	if !result.Success {
		t.Error("execution should have succeeded")
	}
}

// TestStateGas_RevertRefundsStateGasAndTheGasBorrowedForIt covers the exit form
// of a reverting execution: EIP-8037 refunds the state gas it charged, so the
// regular gas that had to be borrowed for it comes back to the caller too.
func TestStateGas_RevertRefundsStateGasAndTheGasBorrowedForIt(t *testing.T) {
	const gas = tosca.Gas(200_000)
	const revertCost = 2 * 3 // < the two pushes of the revert

	result, err := runStoringInterpreter(t, gas, 0, true)
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
	if got, want := result.GasLeft, gas-twoPushesCost-revertCost-sstoreRegularCost; got != want {
		t.Errorf("unexpected gas left, wanted %d, got %d", want, got)
	}
}

// TestStateGas_FailedExecutionKeepsTheReservoirButBurnsRegularGas is the
// counterpart for an exceptional halt, which rolls the state changes back like a
// revert but consumes the regular gas of the execution.
func TestStateGas_FailedExecutionKeepsTheReservoirButBurnsRegularGas(t *testing.T) {
	ctxt := getEmptyContext()
	ctxt.gas = 42
	ctxt.chargedStateGas = 100
	ctxt.spilledStateGas = 20

	result, err := generateResult(statusFailed, &ctxt)
	if err != nil {
		t.Fatalf("failed to generate result: %v", err)
	}
	if result.Success {
		t.Error("a failed execution must not report success")
	}
	if got, want := result.GasLeft, tosca.Gas(0); got != want {
		t.Errorf("unexpected gas left, wanted %d, got %d", want, got)
	}
	if got, want := result.StateGasCharged, tosca.Gas(0); got != want {
		t.Errorf("unexpected state gas charged, wanted %d, got %d", want, got)
	}
}

func TestStateGas_ChargeIsTakenFromTheReservoirBeforeRegularGas(t *testing.T) {
	tests := map[string]struct {
		reservoir   tosca.Gas
		gas         tosca.Gas
		charge      tosca.Gas
		wantErr     error
		wantGas     tosca.Gas
		wantLeft    tosca.Gas
		wantSpilled tosca.Gas
	}{
		"covered by the reservoir": {
			reservoir: 100, gas: 50, charge: 60,
			wantGas: 50, wantLeft: 40, wantSpilled: 0,
		},
		"partially covered": {
			reservoir: 100, gas: 50, charge: 130,
			wantGas: 20, wantLeft: 0, wantSpilled: 30,
		},
		"not covered at all": {
			reservoir: 0, gas: 50, charge: 30,
			wantGas: 20, wantLeft: 0, wantSpilled: 30,
		},
		"beyond the available gas": {
			reservoir: 100, gas: 50, charge: 200,
			wantErr: errOutOfGas,
			wantGas: 50, wantLeft: 100, wantSpilled: 0,
		},
		"negative charge": {
			reservoir: 100, gas: 50, charge: -1,
			wantErr: errOutOfGas,
			wantGas: 50, wantLeft: 100, wantSpilled: 0,
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			ctxt := getEmptyContext()
			ctxt.gas = test.gas
			ctxt.stateGas = test.reservoir

			if got, want := ctxt.useStateGas(test.charge), test.wantErr; got != want {
				t.Fatalf("unexpected error, wanted %v, got %v", want, got)
			}
			if got, want := ctxt.gas, test.wantGas; got != want {
				t.Errorf("unexpected gas, wanted %d, got %d", want, got)
			}
			if got, want := ctxt.stateGas, test.wantLeft; got != want {
				t.Errorf("unexpected reservoir, wanted %d, got %d", want, got)
			}
			if got, want := ctxt.spilledStateGas, test.wantSpilled; got != want {
				t.Errorf("unexpected borrowed gas, wanted %d, got %d", want, got)
			}
		})
	}
}

// TestStateGas_RefundRepaysBorrowedGasBeforeRefillingTheReservoir pins down the
// last-in-first-out order in which a state-gas charge is handed back.
func TestStateGas_RefundRepaysBorrowedGasBeforeRefillingTheReservoir(t *testing.T) {
	ctxt := getEmptyContext()
	ctxt.stateGas = 100

	// A charge of 130 empties the reservoir and borrows the remaining 30.
	ctxt.gas = 30
	if err := ctxt.useStateGas(130); err != nil {
		t.Fatalf("failed to charge state gas: %v", err)
	}

	ctxt.refundStateGas(130)
	if got, want := ctxt.gas, tosca.Gas(30); got != want {
		t.Errorf("borrowed gas was not repaid, wanted %d, got %d", want, got)
	}
	if got, want := ctxt.stateGas, tosca.Gas(100); got != want {
		t.Errorf("reservoir was not refilled, wanted %d, got %d", want, got)
	}
	if got, want := ctxt.spilledStateGas, tosca.Gas(0); got != want {
		t.Errorf("unexpected borrowed gas left, wanted %d, got %d", want, got)
	}
	if got, want := ctxt.chargedStateGas, tosca.Gas(0); got != want {
		t.Errorf("unexpected state gas charged, wanted %d, got %d", want, got)
	}
}

// TestStateGas_NestedCallsCarryTheStateDimension covers the reservoir crossing a
// call boundary: it is forwarded to the callee as a whole, and the charge the
// callee reports is taken over by the caller.
func TestStateGas_NestedCallsCarryTheStateDimension(t *testing.T) {
	const (
		gas       = tosca.Gas(100_000)
		reservoir = tosca.Gas(500)
	)

	tests := map[string]struct {
		charged     tosca.Gas
		wantLeft    tosca.Gas
		wantSpilled tosca.Gas
	}{
		"nothing charged":   {charged: 0, wantLeft: reservoir, wantSpilled: 0},
		"partial charge":    {charged: 200, wantLeft: 300, wantSpilled: 0},
		"reservoir used up": {charged: reservoir, wantLeft: 0, wantSpilled: 0},
		"borrowing gas":     {charged: 700, wantLeft: 0, wantSpilled: 200},
		"net refund":        {charged: -100, wantLeft: reservoir + 100, wantSpilled: 0},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			runContext := tosca.NewMockRunContext(ctrl)
			runContext.EXPECT().AccessAccount(gomock.Any()).Return(tosca.WarmAccess)
			runContext.EXPECT().GetCode(gomock.Any()).AnyTimes()
			runContext.EXPECT().Call(tosca.Call, gomock.Any()).DoAndReturn(
				func(_ tosca.CallKind, parameter tosca.CallParameters) (tosca.CallResult, error) {
					if got, want := parameter.StateGas, reservoir; got != want {
						t.Errorf("unexpected reservoir forwarded, wanted %d, got %d", want, got)
					}
					return tosca.CallResult{
						Success:         true,
						GasLeft:         parameter.Gas,
						StateGasCharged: test.charged,
					}, nil
				})

			ctxt := getEmptyContext()
			ctxt.params.Revision = tosca.R16_Amsterdam
			ctxt.context = runContext
			ctxt.gas = gas
			ctxt.stateGas = reservoir

			zero := *uint256.NewInt(0)
			ctxt.stack = fillStack(zero, zero, zero, zero, zero, zero, zero)
			defer ReturnStack(ctxt.stack)

			if err := genericCall(&ctxt, tosca.Call); err != nil {
				t.Fatalf("genericCall failed: %v", err)
			}
			if got, want := ctxt.chargedStateGas, test.charged; got != want {
				t.Errorf("unexpected state gas charged, wanted %d, got %d", want, got)
			}
			if got, want := ctxt.stateGas, test.wantLeft; got != want {
				t.Errorf("unexpected reservoir left, wanted %d, got %d", want, got)
			}
			if got, want := ctxt.spilledStateGas, test.wantSpilled; got != want {
				t.Errorf("unexpected borrowed gas, wanted %d, got %d", want, got)
			}
		})
	}
}

// runStoringInterpreter executes an SSTORE creating a fresh storage slot at
// Amsterdam, optionally reverting afterwards.
func runStoringInterpreter(t *testing.T, gas, reservoir tosca.Gas, revert bool) (tosca.Result, error) {
	t.Helper()

	code := []byte{
		byte(vm.PUSH1), 1, // < the value to be stored
		byte(vm.PUSH1), 0, // < the slot to store it in
		byte(vm.SSTORE),
	}
	if revert {
		code = append(code,
			byte(vm.PUSH1), 0,
			byte(vm.PUSH1), 0,
			byte(vm.REVERT),
		)
	}

	state := st.NewState(st.NewCode(code))
	defer state.Release()
	state.Revision = tosca.R16_Amsterdam

	parameters := utils.ToVmParameters(state)
	parameters.Gas = gas
	parameters.StateGas = reservoir

	interpreter, err := NewInterpreter(Config{})
	if err != nil {
		t.Fatalf("failed to create interpreter: %v", err)
	}
	return interpreter.Run(parameters)
}
