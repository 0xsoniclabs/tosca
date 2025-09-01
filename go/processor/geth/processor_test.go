// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package geth_processor

import (
	"bytes"
	"math/big"
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/ethereum/go-ethereum/common"
	"github.com/holiman/uint256"
	"github.com/stretchr/testify/require"
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
		"Prague": {
			blockNumber: 7000,
			timestamp:   300,
			revision:    tosca.R14_Prague,
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

			if ((test.revision >= tosca.R14_Prague) != rules.IsPrague) ||
				((test.revision >= tosca.R13_Cancun) != rules.IsCancun) ||
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

func TestTransactionToMessage_BasicFields(t *testing.T) {
	tx := tosca.Transaction{
		Sender:        tosca.Address{0x01},
		Recipient:     &tosca.Address{0x02},
		Nonce:         42,
		Value:         tosca.NewValue(1000),
		GasLimit:      21000,
		GasFeeCap:     tosca.NewValue(50),
		GasTipCap:     tosca.NewValue(2),
		Input:         []byte{0x42, 0x42},
		BlobGasFeeCap: tosca.NewValue(0),
	}
	gasPrice := tosca.NewValue(40)
	blobHashes := []common.Hash{{0xaa}}
	msg := transactionToMessage(tx, gasPrice, blobHashes)

	if msg.From != common.Address(tx.Sender) {
		t.Errorf("From mismatch: got %x, want %x", msg.From, tx.Sender)
	}
	if msg.To == nil || *msg.To != common.Address(*tx.Recipient) {
		t.Errorf("To mismatch: got %v, want %v", msg.To, tx.Recipient)
	}
	if msg.Nonce != tx.Nonce {
		t.Errorf("Nonce mismatch: got %d, want %d", msg.Nonce, tx.Nonce)
	}
	if msg.Value.Cmp(tx.Value.ToBig()) != 0 {
		t.Errorf("Value mismatch: got %v, want %v", msg.Value, tx.Value.ToBig())
	}
	if msg.GasLimit != uint64(tx.GasLimit) {
		t.Errorf("GasLimit mismatch: got %d, want %d", msg.GasLimit, tx.GasLimit)
	}
	if msg.GasPrice.Cmp(gasPrice.ToBig()) != 0 {
		t.Errorf("GasPrice mismatch: got %v, want %v", msg.GasPrice, gasPrice.ToBig())
	}
	if msg.GasFeeCap.Cmp(tx.GasFeeCap.ToBig()) != 0 {
		t.Errorf("GasFeeCap mismatch: got %v, want %v", msg.GasFeeCap, tx.GasFeeCap.ToBig())
	}
	if msg.GasTipCap.Cmp(tx.GasTipCap.ToBig()) != 0 {
		t.Errorf("GasTipCap mismatch: got %v, want %v", msg.GasTipCap, tx.GasTipCap.ToBig())
	}
	if !bytes.Equal(msg.Data, tx.Input) {
		t.Errorf("Input mismatch: got %x, want %x", msg.Data, tx.Input)
	}
	if msg.BlobGasFeeCap.Cmp(tx.BlobGasFeeCap.ToBig()) != 0 {
		t.Errorf("BlobGasFeeCap mismatch: got %v, want %v", msg.BlobGasFeeCap, tx.BlobGasFeeCap.ToBig())
	}
	if len(msg.BlobHashes) != 1 || msg.BlobHashes[0] != blobHashes[0] {
		t.Errorf("BlobHashes mismatch: got %v, want %v", msg.BlobHashes, blobHashes)
	}
}

func TestTransactionToMessage_AccessList(t *testing.T) {
	tx := tosca.Transaction{
		Sender: tosca.Address{0x01},
		AccessList: []tosca.AccessTuple{
			{
				Address: tosca.Address{0x03},
				Keys:    []tosca.Key{{0x42}, {0x43}},
			},
		},
	}
	gasPrice := tosca.NewValue(1)
	msg := transactionToMessage(tx, gasPrice, nil)
	if len(msg.AccessList) != 1 {
		t.Fatalf("expected 1 access tuple, got %d", len(msg.AccessList))
	}
	if msg.AccessList[0].Address != common.Address(tx.AccessList[0].Address) {
		t.Errorf("AccessList address mismatch")
	}
	if len(msg.AccessList[0].StorageKeys) != 2 {
		t.Errorf("expected 2 storage keys, got %d", len(msg.AccessList[0].StorageKeys))
	}
	if msg.AccessList[0].StorageKeys[0] != common.Hash(tx.AccessList[0].Keys[0]) {
		t.Errorf("StorageKey[0] mismatch")
	}
	if msg.AccessList[0].StorageKeys[1] != common.Hash(tx.AccessList[0].Keys[1]) {
		t.Errorf("StorageKey[1] mismatch")
	}
}

func TestTransactionToMessage_AuthorizationList(t *testing.T) {
	tx := tosca.Transaction{
		Sender: [20]byte{0x01},
		AuthorizationList: []tosca.SetCodeAuthorization{
			{
				ChainID: tosca.Word(tosca.NewValue(0x01)),
				Address: tosca.Address{0x02},
				Nonce:   7,
				V:       42,
				R:       tosca.Word(tosca.NewValue(0x03)),
				S:       tosca.Word(tosca.NewValue(0x04)),
			},
		},
	}
	gasPrice := tosca.NewValue(1)
	msg := transactionToMessage(tx, gasPrice, nil)
	if len(msg.SetCodeAuthorizations) != 1 {
		t.Fatalf("expected 1 authorization, got %d", len(msg.SetCodeAuthorizations))
	}
	auth := msg.SetCodeAuthorizations[0]
	if auth.ChainID.Cmp(uint256.NewInt(1)) != 0 {
		t.Errorf("ChainID mismatch: got %v", auth.ChainID)
	}
	if auth.Address != common.Address(tx.AuthorizationList[0].Address) {
		t.Errorf("Address mismatch")
	}
	if auth.Nonce != tx.AuthorizationList[0].Nonce {
		t.Errorf("Nonce mismatch")
	}
	if auth.V != tx.AuthorizationList[0].V {
		t.Errorf("V mismatch")
	}
	if auth.R.Cmp(uint256.NewInt(3)) != 0 {
		t.Errorf("R mismatch: got %v", auth.R)
	}
	if auth.S.Cmp(uint256.NewInt(4)) != 0 {
		t.Errorf("S mismatch: got %v", auth.S)
	}
}

func TestProcessor_ReceiptIsDefaultInitializedInCaseOfError(t *testing.T) {
	ctrl := gomock.NewController(t)
	context := tosca.NewMockTransactionContext(ctrl)
	context.EXPECT().CreateSnapshot()
	context.EXPECT().GetNonce(gomock.Any())
	context.EXPECT().GetCode(gomock.Any())
	context.EXPECT().GetBalance(gomock.Any())
	context.EXPECT().GetBalance(gomock.Any())
	context.EXPECT().SetBalance(gomock.Any(), gomock.Any())
	context.EXPECT().RestoreSnapshot(gomock.Any())

	interpreter := tosca.NewMockInterpreter(ctrl)
	processor := sonicProcessor(interpreter)
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

func TestIsInternal_TransactionsWithZeroSenderAreInternal(t *testing.T) {
	require := require.New(t)
	require.True(isInternal(tosca.Transaction{Sender: tosca.Address{}}))
	require.False(isInternal(tosca.Transaction{Sender: tosca.Address{0x01}}))
}

func TestCalculateGasPrices_InternalTransactions_GetPricesBelowBaseFee(t *testing.T) {
	require := require.New(t)
	baseFee := tosca.NewValue(100)
	gasFeeCap := tosca.NewValue(90)
	gasTipCap := tosca.NewValue(10)
	internal := true

	price, err := calculateGasPrice(baseFee, gasFeeCap, gasTipCap, internal)
	require.NoError(err)
	require.Equal(price, gasFeeCap)
}
