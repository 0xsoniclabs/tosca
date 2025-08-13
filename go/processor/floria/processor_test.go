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
	"reflect"
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/stretchr/testify/require"
	"go.uber.org/mock/gomock"
)

func TestProcessor_NewProcessorReturnsProcessor(t *testing.T) {
	interpreter := tosca.NewMockInterpreter(gomock.NewController(t))
	processor := newFloriaProcessor(interpreter)
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

func TestProcessor_Run_SuccessfulExecution(t *testing.T) {
	ctrl := gomock.NewController(t)
	context := tosca.NewMockRunContext(ctrl)
	interpreter := tosca.NewMockInterpreter(ctrl)

	context.EXPECT().CreateSnapshot().AnyTimes()
	context.EXPECT().GetNonce(gomock.Any()).AnyTimes()
	context.EXPECT().GetCodeHash(gomock.Any()).AnyTimes()
	context.EXPECT().GetBalance(gomock.Any()).AnyTimes()
	context.EXPECT().SetBalance(gomock.Any(), gomock.Any()).AnyTimes()
	context.EXPECT().SetNonce(gomock.Any(), gomock.Any()).AnyTimes()
	context.EXPECT().Call(gomock.Any(), gomock.Any()).AnyTimes()
	context.EXPECT().GetCode(gomock.Any()).AnyTimes()
	interpreter.EXPECT().Run(gomock.Any()).Return(tosca.Result{Success: true}, nil).AnyTimes()
	context.EXPECT().GetLogs().AnyTimes()

	blockParameters := tosca.BlockParameters{}

	transaction := tosca.Transaction{
		Sender:    tosca.Address{1},
		Recipient: &tosca.Address{2},
		GasLimit:  tosca.Gas(1000000),
	}

	processor := newFloriaProcessor(interpreter)
	result, err := processor.Run(blockParameters, transaction, context)
	if err != nil {
		t.Errorf("Run returned an error: %v", err)
	}
	if !result.Success {
		t.Errorf("Run did not succeed")
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
		"all zero": {
			baseFee:   0,
			gasFeeCap: 0,
			gasTipCap: 0,
			expected:  0,
		},
		"high cap no tip": {
			baseFee:   10,
			gasFeeCap: 100,
			gasTipCap: 0,
			expected:  10,
		},
		"equal base, cap and tip": {
			baseFee:   10,
			gasFeeCap: 10,
			gasTipCap: 10,
			expected:  10,
		},
		"cap higher than fee": {
			baseFee:   10,
			gasFeeCap: 20,
			gasTipCap: 5,
			expected:  15,
		},
		"diff smaller than cap": {
			baseFee:   10,
			gasFeeCap: 12,
			gasTipCap: 5,
			expected:  12,
		},
		"equal cap and diff": {
			baseFee:   10,
			gasFeeCap: 15,
			gasTipCap: 5,
			expected:  15,
		},
		"diff bigger than cap": {
			baseFee:   10,
			gasFeeCap: 20,
			gasTipCap: 15,
			expected:  20,
		},
	}
	for testName, test := range tests {
		t.Run(testName, func(t *testing.T) {
			gasPrice, err := calculateGasPrice(tosca.NewValue(test.baseFee), tosca.NewValue(test.gasFeeCap), tosca.NewValue(test.gasTipCap))
			require.NoError(t, err, "calculateGasPrice should not return an error")
			require.Equal(t, tosca.NewValue(test.expected), gasPrice, "calculateGasPrice should return the expected gas price")
		})
	}
}

func TestProcessor_GasPriceCalculationError(t *testing.T) {
	tests := map[string]struct {
		baseFee     uint64
		gasFeeCap   uint64
		gasTipCap   uint64
		errorString string
	}{
		"lower than baseFee": {
			baseFee:     11,
			gasFeeCap:   10,
			gasTipCap:   1,
			errorString: "lower than baseFee",
		},
		"much lower than baseFee": {
			baseFee:     10000,
			gasFeeCap:   10,
			gasTipCap:   1,
			errorString: "lower than baseFee",
		},
		"lower than tipCap": {
			baseFee:     1,
			gasFeeCap:   1,
			gasTipCap:   2,
			errorString: "lower than tipCap",
		},
		"much lower than tipCap": {
			baseFee:     1,
			gasFeeCap:   1,
			gasTipCap:   10000,
			errorString: "lower than tipCap",
		},
	}
	for testName, test := range tests {
		t.Run(testName, func(t *testing.T) {
			_, err := calculateGasPrice(tosca.NewValue(test.baseFee), tosca.NewValue(test.gasFeeCap), tosca.NewValue(test.gasTipCap))
			require.ErrorContains(t, err, test.errorString, "calculateGasPrice should return an error for invalid parameters")
		})
	}
}

func TestProcessor_EOACheckCanHandleEmptyAndZeroHash(t *testing.T) {
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
			err := eoaCheck(tosca.Address{1}, test.codeHash)
			if test.isEOA && err == nil {
				t.Errorf("eoaCheck returned wrong result: %v", err)
			}
		})
	}
}

func TestProcessor_initCodeSizeCheckEnforcesMaximumCodeSize(t *testing.T) {
	recipient := tosca.Address{1}
	tests := map[string]struct {
		revision    tosca.Revision
		recipient   *tosca.Address
		initCode    []byte
		expectError bool
	}{
		"pre shanghai": {
			revision:    tosca.R11_Paris,
			initCode:    make([]byte, maxInitCodeSize+1),
			expectError: false,
		},
		"shanghai": {
			revision:    tosca.R12_Shanghai,
			initCode:    make([]byte, maxInitCodeSize+1),
			expectError: true,
		},
		"under minimum": {
			revision:    tosca.R12_Shanghai,
			initCode:    make([]byte, maxInitCodeSize),
			expectError: false,
		},
		"call": {
			revision:    tosca.R12_Shanghai,
			recipient:   &recipient,
			initCode:    make([]byte, maxInitCodeSize+1),
			expectError: false,
		},
	}

	for testName, test := range tests {
		t.Run(testName, func(t *testing.T) {
			transaction := tosca.Transaction{
				Recipient: test.recipient,
				Input:     test.initCode,
			}
			err := initCodeSizeCheck(test.revision, transaction)
			if test.expectError {
				require.ErrorContains(t, err, "init code too long")
			} else {
				require.NoError(t, err)
			}
		})
	}
}

func TestGasUsed(t *testing.T) {
	tests := map[string]struct {
		transaction     tosca.Transaction
		result          tosca.CallResult
		revision        tosca.Revision
		expectedGasLeft tosca.Gas
		ethCompatible   bool
	}{
		"internalTransaction": {
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
		"nonInternalTransaction": {
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
		"refundPreLondon": {
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
		"refundLondon": {
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
		"refundPostLondon": {
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
		"unsuccessfulResult": {
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
		"ethereumCompatible": {
			transaction: tosca.Transaction{
				Sender:   tosca.Address{1},
				GasLimit: 1000,
			},
			result: tosca.CallResult{
				GasLeft:   500,
				Success:   true,
				GasRefund: 100,
			},
			revision:        tosca.R09_Berlin,
			expectedGasLeft: 600,
			ethCompatible:   true,
		},
		"nonEthereumCompatible": {
			transaction: tosca.Transaction{
				Sender:   tosca.Address{1},
				GasLimit: 1000,
			},
			result: tosca.CallResult{
				GasLeft:   500,
				Success:   true,
				GasRefund: 100,
			},
			revision:        tosca.R09_Berlin,
			expectedGasLeft: 550,
			ethCompatible:   false,
		},
	}

	for testName, test := range tests {
		t.Run(testName, func(t *testing.T) {
			actualGasLeft := calculateGasLeft(test.transaction, test.result, test.revision, test.ethCompatible)

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
		"create Shanghai": {
			recipient:  nil,
			input:      []byte{0, 1, 2, 3, 4, 5, 6, 7, 8, 9},
			accessList: nil,
			expectedGasUsed: TxGasContractCreation + TxDataZeroGasEIP2028 +
				9*TxDataNonZeroGasEIP2028 + InitCodeWordGas,
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			transaction := tosca.Transaction{
				Recipient:  test.recipient,
				Input:      test.input,
				AccessList: test.accessList,
			}

			actualGasUsed := calculateSetupGas(transaction, tosca.R12_Shanghai)
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
	want.CodeAddress = *transaction.Recipient

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
	coinbase := tosca.Address{4}

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

	for _, contract := range getPrecompiledAddresses(tosca.R13_Cancun) {
		context.EXPECT().AccessAccount(contract)
	}
	context.EXPECT().AccessAccount(sender)
	context.EXPECT().AccessAccount(recipient)
	context.EXPECT().AccessAccount(accessListAddress)
	context.EXPECT().AccessStorage(accessListAddress, tosca.Key{1})
	context.EXPECT().AccessStorage(accessListAddress, tosca.Key{2})
	context.EXPECT().AccessAccount(coinbase)

	setUpAccessList(transaction, context, tosca.R13_Cancun, coinbase)
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

	setUpAccessList(transaction, context, tosca.R09_Berlin, tosca.Address{})
}

func TestProcessor_BeforeGasIsBoughtErrorsHaveNoEffect(t *testing.T) {
	baseFee := tosca.NewValue(10)
	gasLimit := tosca.Gas(100000)
	balance := baseFee.Scale(uint64(gasLimit))
	nonce := uint64(24)
	tests := map[string]struct {
		gasFeeCap tosca.Value
		nonce     uint64
		codeHash  tosca.Hash
		blobs     []tosca.Hash
		initCode  tosca.Data
		balance   tosca.Value
	}{
		"failed to calculate gas price": {
			gasFeeCap: tosca.NewValue(5),
		},
		"failed nonce check": {
			gasFeeCap: tosca.NewValue(10),
			nonce:     42,
		},
		"failed EOA check": {
			gasFeeCap: tosca.NewValue(10),
			nonce:     nonce,
			codeHash:  tosca.Hash{1, 2, 3},
		},
		"failed blob check": {
			gasFeeCap: tosca.NewValue(10),
			nonce:     nonce,
			blobs:     []tosca.Hash{{5}},
		},
		"failed init code size check": {
			gasFeeCap: tosca.NewValue(10),
			nonce:     nonce,
			initCode:  make(tosca.Data, maxInitCodeSize+1),
			balance:   balance,
		},
		"failed balance check": {
			gasFeeCap: tosca.NewValue(10),
			nonce:     nonce,
			balance:   tosca.Sub(balance, tosca.NewValue(1)),
		},
		"insufficient gas for set up": {
			gasFeeCap: tosca.NewValue(10),
			nonce:     nonce,
			balance:   balance,
			initCode:  make(tosca.Data, maxInitCodeSize),
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			context := tosca.NewMockRunContext(ctrl)
			interpreter := tosca.NewMockInterpreter(ctrl)

			sender := tosca.Address{1}

			// Only read access to state, getters returning by value.
			context.EXPECT().GetNonce(sender).Return(test.nonce).MaxTimes(1)
			context.EXPECT().GetCodeHash(sender).Return(test.codeHash).MaxTimes(1)
			context.EXPECT().GetBalance(sender).Return(test.balance).MaxTimes(1)

			blockParameters := tosca.BlockParameters{
				Revision: tosca.R12_Shanghai,
				BaseFee:  baseFee,
			}

			transaction := tosca.Transaction{
				Sender:     sender,
				GasLimit:   gasLimit,
				GasFeeCap:  test.gasFeeCap,
				Nonce:      nonce,
				BlobHashes: test.blobs,
				Input:      test.initCode,
			}

			processor := newFloriaProcessor(interpreter)
			_, err := processor.Run(blockParameters, transaction, context)
			require.ErrorContains(t, err, name)
		})
	}
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

	processor := newFloriaProcessor(interpreter)
	_, err := processor.Run(blockParameters, transaction, context)
	require.ErrorContains(t, err, "missing blob hashes")
}

func TestProcessor_BalanceCheckReturnsErrors(t *testing.T) {
	tests := map[string]struct {
		gasLimit      tosca.Gas
		expectedError error
	}{
		"no error": {
			gasLimit:      tosca.Gas(1),
			expectedError: nil,
		},
		"gas overflow": {
			gasLimit:      tosca.Gas(math.MaxInt64),
			expectedError: fmt.Errorf("capGas overflow"),
		},
		"insufficient balance": {
			gasLimit:      tosca.Gas(100),
			expectedError: fmt.Errorf("insufficient balance"),
		},
	}

	for testName, test := range tests {
		t.Run(testName, func(t *testing.T) {
			hugeValue := tosca.NewValue(math.MaxUint64/1000, math.MaxUint64, math.MaxUint64, math.MaxUint64)
			transaction := tosca.Transaction{
				GasFeeCap: hugeValue,
				GasLimit:  test.gasLimit,
			}
			balance := hugeValue
			gasPrice := tosca.NewValue(5)
			err := balanceCheck(gasPrice, transaction, balance, true)
			if test.expectedError == nil {
				require.NoError(t, err, "balanceCheck should not return an error")
			} else {
				require.ErrorContains(t, err, test.expectedError.Error(), "balanceCheck should return the expected error")
			}
		})
	}
}

func TestProcessor_NonEthCompatibleBalanceCheckIgnoresGasFeeCapAndValue(t *testing.T) {
	tests := map[string]struct {
		value         tosca.Value
		gasFeeCap     tosca.Value
		ethCompatible bool
	}{
		"value": {
			value:         tosca.NewValue(100),
			ethCompatible: false,
		},
		"value-eth": {
			value:         tosca.NewValue(100),
			ethCompatible: true,
		},
		"gasFeeCap": {
			gasFeeCap:     tosca.NewValue(100),
			ethCompatible: false,
		},
		"gasFeeCap-eth": {
			gasFeeCap:     tosca.NewValue(100),
			ethCompatible: true,
		},
	}

	for testName, test := range tests {
		t.Run(testName, func(t *testing.T) {
			transaction := tosca.Transaction{
				GasFeeCap: test.gasFeeCap,
				GasLimit:  tosca.Gas(10),
				Value:     test.value,
			}

			gasPrice := tosca.NewValue(1)
			err := balanceCheck(gasPrice, transaction, tosca.NewValue(10), test.ethCompatible)
			if test.ethCompatible {
				require.ErrorContains(t, err, "insufficient balance")
			} else {
				require.NoError(t, err, "balanceCheck should not return an error")
			}
		})
	}
}

func TestProcessor_BalanceCheckCalculatesCapGasCorrectly(t *testing.T) {
	tests := map[string]struct {
		gasFeeCap     tosca.Value
		gasPrice      tosca.Value
		value         tosca.Value
		ethCompatible bool
		blobHashes    []tosca.Hash
		checkValue    tosca.Value
	}{
		"baseline": {
			gasPrice:   tosca.NewValue(10),
			value:      tosca.NewValue(10),
			checkValue: tosca.NewValue(100),
		},
		"with gas fee cap": {
			gasFeeCap:     tosca.NewValue(20),
			ethCompatible: true,
			checkValue:    tosca.NewValue(200),
		},
		"ethereum compatible": {
			gasPrice:      tosca.NewValue(10),
			value:         tosca.NewValue(10),
			ethCompatible: true,
			checkValue:    tosca.NewValue(110),
		},
		"with blobhashes": {
			gasPrice:   tosca.NewValue(10),
			blobHashes: []tosca.Hash{{0x01}, {0x02}},
			checkValue: tosca.NewValue(262244),
		},
	}

	for testName, test := range tests {
		t.Run(testName, func(t *testing.T) {

			transaction := tosca.Transaction{
				GasFeeCap:     test.gasFeeCap,
				BlobGasFeeCap: tosca.NewValue(1),
				BlobHashes:    test.blobHashes,
				GasLimit:      tosca.Gas(10),
				Value:         test.value,
			}
			balance := tosca.NewValue(1)

			err := balanceCheck(test.gasPrice, transaction, balance, test.ethCompatible)

			// this test is using the error message to check that the gas cap was correctly calculated.
			errorMessage := fmt.Sprintf("insufficient balance: 1 < %v", test.checkValue)
			require.ErrorContains(t, err, errorMessage, "balanceCheck should return an error with the correct cap gas")
		})
	}
}

func TestProcessor_BuyGas_AccountsForBlobs(t *testing.T) {
	senderBalance := tosca.NewValue(uint64(1000))
	gasPrice := tosca.NewValue(uint64(10))
	blobGasPrice := tosca.NewValue(uint64(15))
	gasLimit := tosca.Gas(10)
	baseCost := gasPrice.Scale(uint64(gasLimit))

	tests := map[string]struct {
		blobHashes     []tosca.Hash
		expectedUpdate tosca.Value
	}{
		"nil blobs": {
			blobHashes:     nil,
			expectedUpdate: baseCost,
		},
		"empty blobs": {
			blobHashes:     []tosca.Hash{},
			expectedUpdate: baseCost,
		},
		"with blobs": {
			blobHashes:     []tosca.Hash{{0x01}, {0x02}},
			expectedUpdate: tosca.Add(baseCost, blobGasPrice.Scale(uint64(2)*BlobTxBlobGasPerBlob)),
		},
	}

	for testName, test := range tests {
		t.Run(testName, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			context := tosca.NewMockTransactionContext(ctrl)

			sender := tosca.Address{1}
			transaction := tosca.Transaction{
				Sender:     sender,
				GasLimit:   gasLimit,
				BlobHashes: test.blobHashes,
			}

			context.EXPECT().GetBalance(sender).Return(senderBalance)
			context.EXPECT().SetBalance(sender, tosca.Sub(senderBalance, test.expectedUpdate))

			buyGas(transaction, gasPrice, blobGasPrice, context)
		})
	}
}

func TestProcessor_PaymentToCoinbase(t *testing.T) {
	baseFee := tosca.NewValue(10)
	tests := map[string]struct {
		revision tosca.Revision
		gasPrice tosca.Value
		payment  tosca.Value
	}{
		"pre London tip is gasPrice": {
			revision: tosca.R09_Berlin,
			gasPrice: tosca.NewValue(12),
			payment:  tosca.NewValue(12),
		},
		"post London tip is gasPrice minus baseFee": {
			revision: tosca.R10_London,
			gasPrice: tosca.NewValue(12),
			payment:  tosca.NewValue(2), // 12 - 10
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			context := tosca.NewMockTransactionContext(ctrl)

			initialBalance := tosca.NewValue(100)
			coinbase := tosca.Address{42}
			gasUsed := tosca.Gas(1)
			blockParameters := tosca.BlockParameters{
				Coinbase: coinbase,
				BaseFee:  baseFee,
				Revision: test.revision,
			}

			context.EXPECT().GetBalance(coinbase).Return(initialBalance)
			context.EXPECT().SetBalance(coinbase, tosca.Add(initialBalance, test.payment))

			paymentToCoinbase(test.gasPrice, gasUsed, blockParameters, context)
		})
	}
}
