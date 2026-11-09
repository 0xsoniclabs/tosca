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
	"slices"
	"testing"

	"github.com/0xsoniclabs/tosca/go/geth_adapter"
	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/ethereum/go-ethereum/common"
	geth "github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/params"
	"go.uber.org/mock/gomock"
)

func TestProcessor_ReceiptIsDefaultInitializedInCaseOfError(t *testing.T) {
	ctrl := gomock.NewController(t)
	context := tosca.NewMockTransactionContext(ctrl)
	context.EXPECT().GetNonce(gomock.Any())
	context.EXPECT().GetCodeHash(gomock.Any())
	context.EXPECT().GetBalance(gomock.Any())
	context.EXPECT().SetBalance(gomock.Any(), gomock.Any())

	interpreter := tosca.NewMockInterpreter(ctrl)
	processor := newProcessor(interpreter)
	blockParams := tosca.BlockParameters{}
	transaction := tosca.Transaction{}
	receipt, err := processor.Run(blockParams, transaction, context)
	if err == nil {
		t.Errorf("expected error, got nil")
	}
	if receipt.Success || receipt.GasUsed != 0 || receipt.BlobGasUsed != 0 ||
		receipt.Output != nil || receipt.ContractAddress != nil || len(receipt.Logs) != 0 {
		t.Errorf("expected empty receipt, got %v", receipt)
	}
}

func TestPreCompiledStateContract_ReturnsStateGasUnchanged(t *testing.T) {
	const executionGas, stateGas = 10_000, 300

	account := common.Address{1}
	setBalance := slices.Concat(setBalanceMethodID, common.LeftPadBytes(account[:], 32), make([]byte, 32))

	tests := map[string]struct {
		caller  common.Address
		wantErr error
		want    geth.GasBudget
	}{
		"success": {
			caller: driverAddress,
			want:   geth.NewGasBudget(executionGas-params.CallValueTransferGas, stateGas),
		},
		"failure": {
			caller:  common.Address{2},
			wantErr: geth.ErrExecutionReverted,
			want:    geth.NewGasBudget(0, stateGas),
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			context := tosca.NewMockTransactionContext(ctrl)
			context.EXPECT().GetBalance(tosca.Address(account)).AnyTimes()
			context.EXPECT().SetBalance(tosca.Address(account), tosca.Value{}).AnyTimes()

			_, got, err := preCompiledStateContract{}.Run(
				geth_adapter.NewStateDB(context), geth.BlockContext{}, geth.TxContext{},
				test.caller, setBalance, geth.NewGasBudget(executionGas, stateGas),
			)
			if err != test.wantErr {
				t.Errorf("unexpected error, wanted %v, got %v", test.wantErr, err)
			}
			if got != test.want {
				t.Errorf("unexpected gas budget, wanted %v, got %v", test.want, got)
			}
		})
	}
}
