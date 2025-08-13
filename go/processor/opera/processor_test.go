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

	"github.com/0xsoniclabs/tosca/go/tosca"
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
