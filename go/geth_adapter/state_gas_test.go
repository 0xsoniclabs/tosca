// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package geth_adapter

import (
	"math/big"
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/ethereum/go-ethereum/common"
	geth "github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/params"
	"go.uber.org/mock/gomock"
)

// TestGethAdapter_StateGasChargeIsSplitOverTheReservoir covers the direction in
// which geth runs a Tosca interpreter: the reservoir of the frame is offered to
// the interpreter, and the charge it reports has to be split back over the
// reservoir and the regular gas it borrowed, so that geth's own exit and absorb
// paths can refill it correctly.
func TestGethAdapter_StateGasChargeIsSplitOverTheReservoir(t *testing.T) {
	const gas = 1000

	tests := map[string]struct {
		reservoir     uint64
		charged       tosca.Gas
		wantState     uint64
		wantSpilled   uint64
		wantUsedState int64
	}{
		"nothing charged leaves the reservoir alone": {
			reservoir: 500, charged: 0,
			wantState: 500, wantSpilled: 0, wantUsedState: 0,
		},
		"charge covered by the reservoir": {
			reservoir: 500, charged: 200,
			wantState: 300, wantSpilled: 0, wantUsedState: 200,
		},
		"charge exhausting the reservoir": {
			reservoir: 500, charged: 500,
			wantState: 0, wantSpilled: 0, wantUsedState: 500,
		},
		"charge borrowing regular gas": {
			reservoir: 500, charged: 700,
			wantState: 0, wantSpilled: 200, wantUsedState: 700,
		},
		"charge without a reservoir": {
			reservoir: 0, charged: 300,
			wantState: 0, wantSpilled: 300, wantUsedState: 300,
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			stateDb := NewMockStateDb(ctrl)
			interpreter := tosca.NewMockInterpreter(ctrl)

			refundShift := uint64(1 << 60)
			stateDb.EXPECT().AddRefund(refundShift)
			stateDb.EXPECT().AddRefund(uint64(0))
			stateDb.EXPECT().GetRefund().Return(refundShift)
			stateDb.EXPECT().SubRefund(refundShift)

			blockParameters := geth.BlockContext{BlockNumber: big.NewInt(24)}
			chainConfig := &params.ChainConfig{ChainID: big.NewInt(42), IstanbulBlock: big.NewInt(23)}
			evm := geth.NewEVM(blockParameters, stateDb, chainConfig, geth.Config{})
			adapter := &gethInterpreterAdapter{evm: evm, interpreter: interpreter}

			interpreter.EXPECT().Run(gomock.Any()).DoAndReturn(
				func(parameters tosca.Parameters) (tosca.Result, error) {
					if got, want := parameters.StateGas, tosca.Gas(test.reservoir); got != want {
						t.Errorf("unexpected reservoir offered, wanted %d, got %d", want, got)
					}
					return tosca.Result{
						Success:         true,
						GasLeft:         parameters.Gas,
						StateGasCharged: test.charged,
					}, nil
				})

			budget := geth.NewGasBudget(gas, test.reservoir)
			contract := geth.NewContract(common.Address{}, common.Address{}, nil, budget, nil)

			if _, err := adapter.Interpret(contract, nil, false); err != nil {
				t.Fatalf("unexpected error: %v", err)
			}

			if got, want := contract.Gas.StateGas, test.wantState; got != want {
				t.Errorf("unexpected reservoir left, wanted %d, got %d", want, got)
			}
			if got, want := contract.Gas.Spilled, test.wantSpilled; got != want {
				t.Errorf("unexpected borrowed regular gas, wanted %d, got %d", want, got)
			}
			if got, want := contract.Gas.UsedStateGas, test.wantUsedState; got != want {
				t.Errorf("unexpected state gas used, wanted %d, got %d", want, got)
			}
		})
	}
}
