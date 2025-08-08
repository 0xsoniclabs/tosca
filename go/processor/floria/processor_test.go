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
	"math"
	"reflect"
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/stretchr/testify/require"
	"go.uber.org/mock/gomock"
)

func TestProcessor_NewProcessorReturnsProcessor(t *testing.T) {
	interpreter := tosca.NewMockInterpreter(gomock.NewController(t))
	processor := newProcessor(interpreter)
	if processor == nil {
		t.Errorf("newProcessor returned nil")
	}
}

func TestProcessorRegistry_InitProcessor(t *testing.T) {
	processorFactories := tosca.GetAllRegisteredProcessorFactories()
	if len(processorFactories) == 0 {
		t.Errorf("No processor factories found")
	}

	processor := tosca.GetProcessorFactory("floria")
	if processor == nil {
		t.Errorf("Floria processor factory not found")
	}
}

func TestProcessor_HandleNonce(t *testing.T) {
	ctrl := gomock.NewController(t)
	context := tosca.NewMockTransactionContext(ctrl)

	context.EXPECT().GetNonce(tosca.Address{1}).Return(uint64(9))

	transaction := tosca.Transaction{
		Sender: tosca.Address{1},
		Nonce:  9,
	}

	err := nonceCheck(transaction.Nonce, context.GetNonce(transaction.Sender))
	if err != nil {
		t.Errorf("nonceCheck returned an error: %v", err)
	}
}

func TestProcessor_NonceOverflowIsDetected(t *testing.T) {
	err := nonceCheck(math.MaxUint64, math.MaxUint64)
	if err == nil {
		t.Errorf("nonceCheck did not spot nonce overflow")
	}
}

func TestProcessor_NonceMissMatch(t *testing.T) {
	err := nonceCheck(uint64(10), uint64(42))
	if err == nil {
		t.Errorf("nonceCheck did not spot nonce miss match")
	}
}

func TestProcessor_GasPriceCalculation(t *testing.T) {
	tests := map[string]struct {
		baseFee   uint64
		gasFeeCap uint64
		gasTipCap uint64
		expected  uint64
	}{
		"zero": {
			baseFee:   0,
			gasFeeCap: 0,
			gasTipCap: 0,
			expected:  0,
		},
		"highCapNoTip": {
			baseFee:   10,
			gasFeeCap: 100,
			gasTipCap: 0,
			expected:  10,
		},
		"lowCapHighTip": {
			baseFee:   10,
			gasFeeCap: 10,
			gasTipCap: 100,
			expected:  10,
		},
		"capTipEqual": {
			baseFee:   10,
			gasFeeCap: 10,
			gasTipCap: 10,
			expected:  10,
		},
		"capHigherThanFee": {
			baseFee:   10,
			gasFeeCap: 20,
			gasTipCap: 5,
			expected:  15,
		},
		"partOfTip": {
			baseFee:   10,
			gasFeeCap: 12,
			gasTipCap: 5,
			expected:  12,
		},
	}
	for testName, test := range tests {
		t.Run(testName, func(t *testing.T) {
			gasPrice, err := calculateGasPrice(tosca.NewValue(test.baseFee), tosca.NewValue(test.gasFeeCap), tosca.NewValue(test.gasTipCap))
			if err != nil {
				t.Fatalf("calculateGasPrice returned an error: %v", err)
			}
			if gasPrice.Cmp(tosca.NewValue(test.expected)) != 0 {
				t.Errorf("calculateGasPrice returned wrong result, want: %v, got: %v", test.expected, gasPrice)
			}
		})
	}
}

func TestProcessor_GasPriceCalculationError(t *testing.T) {
	_, err := calculateGasPrice(tosca.NewValue(10), tosca.NewValue(5), tosca.NewValue(10))
	if err == nil {
		t.Errorf("calculateGasPrice did not return an error")
	}
}

func TestProcessor_EoaCheck(t *testing.T) {
	tests := map[string]struct {
		codeHash tosca.Hash
		isEOA    bool
	}{
		"empty": {
			tosca.Hash{},
			false,
		},
		"emptyHash": {
			emptyCodeHash,
			false,
		},
		"nonEmpty": {
			tosca.Hash{1, 2, 3},
			true,
		},
	}

	for testName, test := range tests {
		t.Run(testName, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			context := tosca.NewMockTransactionContext(ctrl)

			context.EXPECT().GetCodeHash(tosca.Address{1}).Return(test.codeHash)

			err := eoaCheck(tosca.Address{1}, context)
			if test.isEOA && err == nil {
				t.Errorf("eoaCheck returned wrong result: %v", err)
			}
		})
	}
}

func TestProcessor_BuyGas(t *testing.T) {
	balance := uint64(1000000)
	gasLimit := uint64(100)
	gasPrice := uint64(2)
	blobGasPrice := uint64(1)

	transaction := tosca.Transaction{
		Sender:     tosca.Address{1},
		GasLimit:   tosca.Gas(gasLimit),
		BlobHashes: []tosca.Hash{{0x01}},
	}

	newBalance := balance - (gasLimit*gasPrice +
		blobGasPrice*uint64(len(transaction.BlobHashes))*BlobTxBlobGasPerBlob)

	ctrl := gomock.NewController(t)
	context := tosca.NewMockTransactionContext(ctrl)
	context.EXPECT().GetBalance(transaction.Sender).Return(tosca.NewValue(balance))
	context.EXPECT().SetBalance(transaction.Sender, tosca.NewValue(newBalance))

	err := buyGas(transaction, context, tosca.NewValue(gasPrice), tosca.NewValue(blobGasPrice))
	if err != nil {
		t.Errorf("buyGas returned an error: %v", err)
	}
}

func TestProcessor_BuyGasInsufficientBalance(t *testing.T) {
	balance := uint64(100)
	gasLimit := uint64(100)
	gasPrice := uint64(2)

	transaction := tosca.Transaction{
		Sender:   tosca.Address{1},
		GasLimit: tosca.Gas(gasLimit),
	}

	ctrl := gomock.NewController(t)
	context := tosca.NewMockTransactionContext(ctrl)
	context.EXPECT().GetBalance(transaction.Sender).Return(tosca.NewValue(balance))

	err := buyGas(transaction, context, tosca.NewValue(gasPrice), tosca.NewValue(0))
	if err == nil {
		t.Errorf("buyGas did not fail with insufficient balance")
	}
}

func TestGasUsed(t *testing.T) {
	tests := map[string]struct {
		transaction     tosca.Transaction
		result          tosca.CallResult
		revision        tosca.Revision
		expectedGasLeft tosca.Gas
	}{
		"InternalTransaction": {
			transaction: tosca.Transaction{
				Sender:   tosca.Address{},
				GasLimit: 1000,
			},
			result: tosca.CallResult{
				GasLeft:   500,
				Success:   true,
				GasRefund: 0,
			},
			revision:        tosca.R10_London,
			expectedGasLeft: 500,
		},
		"NonInternalTransaction": {
			transaction: tosca.Transaction{
				Sender:   tosca.Address{1},
				GasLimit: 1000,
			},
			result: tosca.CallResult{
				GasLeft:   500,
				Success:   true,
				GasRefund: 0,
			},
			revision:        tosca.R10_London,
			expectedGasLeft: 450,
		},
		"RefundPreLondon": {
			transaction: tosca.Transaction{
				Sender:   tosca.Address{},
				GasLimit: 1000,
			},
			result: tosca.CallResult{
				GasLeft:   500,
				Success:   true,
				GasRefund: 300,
			},
			revision:        tosca.R09_Berlin,
			expectedGasLeft: 750,
		},
		"RefundLondon": {
			transaction: tosca.Transaction{
				Sender:   tosca.Address{},
				GasLimit: 1000,
			},
			result: tosca.CallResult{
				GasLeft:   500,
				Success:   true,
				GasRefund: 300,
			},
			revision:        tosca.R10_London,
			expectedGasLeft: 600,
		},
		"RefundPostLondon": {
			transaction: tosca.Transaction{
				Sender:   tosca.Address{},
				GasLimit: 1000,
			},
			result: tosca.CallResult{
				GasLeft:   500,
				Success:   true,
				GasRefund: 300,
			},
			revision:        tosca.R13_Cancun,
			expectedGasLeft: 600,
		},
		"smallRefund": {
			transaction: tosca.Transaction{
				Sender:   tosca.Address{},
				GasLimit: 1000,
			},
			result: tosca.CallResult{
				GasLeft:   500,
				Success:   true,
				GasRefund: 5,
			},
			revision:        tosca.R10_London,
			expectedGasLeft: 505,
		},
		"UnsuccessfulResult": {
			transaction: tosca.Transaction{
				Sender:   tosca.Address{},
				GasLimit: 1000,
			},
			result: tosca.CallResult{
				GasLeft:   0,
				Success:   false,
				GasRefund: 500,
			},
			revision:        tosca.R10_London,
			expectedGasLeft: 0,
		},
	}

	for testName, test := range tests {
		t.Run(testName, func(t *testing.T) {
			actualGasLeft := calculateGasLeft(test.transaction, test.result, test.revision)

			if actualGasLeft != test.expectedGasLeft {
				t.Errorf("gasUsed returned incorrect result, got: %d, want: %d", actualGasLeft, test.expectedGasLeft)
			}
		})
	}
}

func TestProcessor_RefundGas(t *testing.T) {
	gasPrice := 5
	gasLeft := 50
	senderBalance := 1000

	sender := tosca.Address{1}

	ctrl := gomock.NewController(t)
	context := tosca.NewMockTransactionContext(ctrl)

	context.EXPECT().GetBalance(sender).Return(tosca.NewValue(uint64(senderBalance)))
	context.EXPECT().SetBalance(sender, tosca.NewValue(uint64(senderBalance+gasLeft*gasPrice)))

	refundGas(context, sender, tosca.NewValue(uint64(gasPrice)), tosca.Gas(gasLeft))
}

func TestProcessor_SetupGasBilling(t *testing.T) {
	tests := map[string]struct {
		recipient       *tosca.Address
		input           []byte
		accessList      []tosca.AccessTuple
		expectedGasUsed tosca.Gas
	}{
		"creation": {
			recipient:       nil,
			input:           []byte{},
			accessList:      nil,
			expectedGasUsed: TxGasContractCreation,
		},
		"call": {
			recipient:       &tosca.Address{1},
			input:           []byte{},
			accessList:      nil,
			expectedGasUsed: TxGas,
		},
		"inputZeros": {
			recipient:       &tosca.Address{1},
			input:           []byte{0, 0, 0, 0, 0, 0, 0, 0, 0, 0},
			accessList:      nil,
			expectedGasUsed: TxGas + 10*TxDataZeroGasEIP2028,
		},
		"inputNonZeros": {
			recipient:       &tosca.Address{1},
			input:           []byte{1, 2, 3, 4, 5, 6, 7, 8, 9, 10},
			accessList:      nil,
			expectedGasUsed: TxGas + 10*TxDataNonZeroGasEIP2028,
		},
		"accessList": {
			recipient: &tosca.Address{1},
			input:     []byte{},
			accessList: []tosca.AccessTuple{
				{
					Address: tosca.Address{1},
					Keys:    []tosca.Key{{1}, {2}, {3}},
				},
			},
			expectedGasUsed: TxGas + TxAccessListAddressGas + 3*TxAccessListStorageKeyGas,
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			transaction := tosca.Transaction{
				Recipient:  test.recipient,
				Input:      test.input,
				AccessList: test.accessList,
			}

			actualGasUsed := calculateSetupGas(transaction)
			if actualGasUsed != test.expectedGasUsed {
				t.Errorf("setupGasBilling returned incorrect gas used, got: %d, want: %d", actualGasUsed, test.expectedGasUsed)
			}
		})
	}
}

func TestProcessor_CallKind(t *testing.T) {
	tests := map[string]struct {
		recipient *tosca.Address
		kind      tosca.CallKind
	}{
		"call": {
			recipient: &tosca.Address{2},
			kind:      tosca.Call,
		},
		"create": {
			recipient: nil,
			kind:      tosca.Create,
		},
	}
	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			transaction := tosca.Transaction{
				Sender:    tosca.Address{1},
				Recipient: test.recipient,
			}
			if callKind(transaction) != test.kind {
				t.Errorf("callKind returned incorrect result: %v", callKind(transaction))
			}
		})
	}
}

func TestProcessor_CallParameters(t *testing.T) {
	transaction := tosca.Transaction{
		Sender: tosca.Address{1},
		Input:  []byte{1, 2, 3},
		Value:  tosca.NewValue(100),
	}
	gas := tosca.Gas(1000)

	want := tosca.CallParameters{
		Sender: transaction.Sender,
		Input:  transaction.Input,
		Value:  transaction.Value,
		Gas:    gas,
	}

	got := callParameters(transaction, gas)
	if !reflect.DeepEqual(got, want) {
		t.Errorf("callParameters returned incorrect result: %v", got)

	}

	transaction.Recipient = &tosca.Address{2}
	want.Recipient = *transaction.Recipient

	got = callParameters(transaction, gas)
	if !reflect.DeepEqual(got, want) {
		t.Errorf("callParameters returned incorrect result: %v", got)

	}
}

func TestProcessor_SetUpAccessList(t *testing.T) {
	ctrl := gomock.NewController(t)
	context := tosca.NewMockTransactionContext(ctrl)

	sender := tosca.Address{1}
	recipient := tosca.Address{2}
	accessListAddress := tosca.Address{3}

	transaction := tosca.Transaction{
		Sender:    sender,
		Recipient: &recipient,
		AccessList: []tosca.AccessTuple{
			{
				Address: accessListAddress,
				Keys:    []tosca.Key{{1}, {2}},
			},
		},
	}

	for _, contract := range getPrecompiledAddresses(tosca.R09_Berlin) {
		context.EXPECT().AccessAccount(contract)
	}
	context.EXPECT().AccessAccount(sender)
	context.EXPECT().AccessAccount(recipient)
	context.EXPECT().AccessAccount(accessListAddress)
	context.EXPECT().AccessStorage(accessListAddress, tosca.Key{1})
	context.EXPECT().AccessStorage(accessListAddress, tosca.Key{2})

	setUpAccessList(transaction, context, tosca.R09_Berlin)
}

func TestProcessor_AccessListIsNotCreatedIfTransactionHasNone(t *testing.T) {
	ctrl := gomock.NewController(t)
	context := tosca.NewMockTransactionContext(ctrl)
	// No calls to context

	sender := tosca.Address{1}
	recipient := tosca.Address{2}

	transaction := tosca.Transaction{
		Sender:    sender,
		Recipient: &recipient,
	}

	setUpAccessList(transaction, context, tosca.R09_Berlin)
}

func TestProcessor_blobCheckReturnsErrors(t *testing.T) {
	tests := map[string]struct {
		transaction tosca.Transaction
		blockParams tosca.BlockParameters
		errorString string
	}{
		"valid blob transaction pre cancun": {
			transaction: tosca.Transaction{
				Recipient:  &tosca.Address{1},
				BlobHashes: nil,
			},
			blockParams: tosca.BlockParameters{Revision: tosca.R12_Shanghai},
			errorString: "",
		},
		"valid blob transaction cancun": {
			transaction: tosca.Transaction{
				Recipient:     &tosca.Address{1},
				BlobHashes:    []tosca.Hash{{1}},
				BlobGasFeeCap: tosca.NewValue(10),
			},
			blockParams: tosca.BlockParameters{
				Revision:    tosca.R13_Cancun,
				BlobBaseFee: tosca.NewValue(1),
			},
			errorString: "",
		},
		"blob transaction without recipient": {
			transaction: tosca.Transaction{
				BlobHashes: []tosca.Hash{{1}},
			},
			blockParams: tosca.BlockParameters{Revision: tosca.R13_Cancun},
			errorString: "blob transaction without recipient",
		},
		"blob transaction with empty blob hashes": {
			transaction: tosca.Transaction{
				Recipient:  &tosca.Address{1},
				BlobHashes: []tosca.Hash{},
			},
			blockParams: tosca.BlockParameters{Revision: tosca.R13_Cancun},
			errorString: "missing blob hashes",
		},
		"blob transaction with invalid hash version": {
			transaction: tosca.Transaction{
				Recipient:  &tosca.Address{1},
				BlobHashes: []tosca.Hash{{5}},
			},
			blockParams: tosca.BlockParameters{Revision: tosca.R12_Shanghai},
			errorString: "blob with invalid hash version",
		},
		"blobGasFeeCap smaller than blobBaseFee": {
			transaction: tosca.Transaction{
				Recipient:     &tosca.Address{1},
				BlobHashes:    []tosca.Hash{{1}},
				BlobGasFeeCap: tosca.NewValue(100),
			},
			blockParams: tosca.BlockParameters{
				Revision:    tosca.R13_Cancun,
				BlobBaseFee: tosca.NewValue(200),
			},
			errorString: "blobGasFeeCap is lower than blobBaseFee",
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			err := checkBlobs(test.transaction, test.blockParams)
			if test.errorString != "" {
				require.ErrorContains(t, err, test.errorString)
			} else {
				require.NoError(t, err, "checkBlobs should not return an error")
			}
		})
	}
}

func TestProcessor_Run_BlobTransactionWithoutBlobsIsUnsuccessful(t *testing.T) {
	ctrl := gomock.NewController(t)
	context := tosca.NewMockRunContext(ctrl)
	interpreter := tosca.NewMockInterpreter(ctrl)

	context.EXPECT().GetNonce(gomock.Any())
	context.EXPECT().GetCodeHash(gomock.Any())

	blockParameters := tosca.BlockParameters{}

	transaction := tosca.Transaction{
		Sender:     tosca.Address{1},
		Recipient:  &tosca.Address{2},
		GasLimit:   tosca.Gas(1000000),
		BlobHashes: []tosca.Hash{}, // No blobs but not nil
	}

	processor := newProcessor(interpreter)
	result, err := processor.Run(blockParameters, transaction, context)
	if err != nil {
		t.Errorf("Run returned an error: %v", err)
	}
	if result.Success {
		t.Errorf("Run should not succeed for blob transaction without blobs")
	}
}
