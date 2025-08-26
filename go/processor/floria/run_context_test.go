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
	"testing"

	test_utils "github.com/0xsoniclabs/tosca/go/processor"
	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/crypto"
	"github.com/stretchr/testify/require"
	"go.uber.org/mock/gomock"
)

func TestCalls_InterpreterResultIsHandledCorrectly(t *testing.T) {
	tests := map[string]struct {
		setup   func(interpreter *tosca.MockInterpreter)
		success bool
		output  []byte
	}{
		"successful": {
			setup: func(interpreter *tosca.MockInterpreter) {
				interpreter.EXPECT().Run(gomock.Any()).Return(tosca.Result{Success: true}, nil)
			},
			success: true,
		},
		"failed": {
			setup: func(interpreter *tosca.MockInterpreter) {
				interpreter.EXPECT().Run(gomock.Any()).Return(tosca.Result{Success: false}, nil)
			},
			success: false,
		},
		"output": {
			setup: func(interpreter *tosca.MockInterpreter) {
				interpreter.EXPECT().Run(gomock.Any()).Return(tosca.Result{Success: true, Output: []byte("some output")}, nil)
			},
			success: true,
			output:  []byte("some output"),
		},
	}

	ctrl := gomock.NewController(t)
	context := tosca.NewMockTransactionContext(ctrl)
	interpreter := tosca.NewMockInterpreter(ctrl)

	runContext := runContext{
		context,
		interpreter,
		tosca.BlockParameters{},
		tosca.TransactionParameters{},
		0,
		false,
	}

	params := tosca.CallParameters{
		Sender:    tosca.Address{1},
		Recipient: tosca.Address{2},
		Value:     tosca.NewValue(0),
		Gas:       1000,
		Input:     []byte{},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			context.EXPECT().GetCodeHash(params.Recipient).Return(tosca.Hash{})
			context.EXPECT().GetCode(params.Recipient).Return([]byte{})
			context.EXPECT().CreateSnapshot()
			context.EXPECT().RestoreSnapshot(gomock.Any()).AnyTimes()

			test.setup(interpreter)

			result, err := runContext.Call(tosca.Call, params)
			if err != nil {
				t.Errorf("Call returned an unexpected error: %v", err)
			}
			if result.Success != test.success {
				t.Errorf("Unexpected success value from interpreter call")
			}
			if string(result.Output) != string(test.output) {
				t.Errorf("Unexpected output value from interpreter call")
			}
		})
	}
}

func TestCall_TransferValueInCall(t *testing.T) {
	ctrl := gomock.NewController(t)
	context := tosca.NewMockTransactionContext(ctrl)
	interpreter := tosca.NewMockInterpreter(ctrl)
	runContext := runContext{
		context,
		interpreter,
		tosca.BlockParameters{},
		tosca.TransactionParameters{},
		0,
		false,
	}

	params := tosca.CallParameters{
		Sender:    tosca.Address{1},
		Recipient: tosca.Address{2},
		Value:     tosca.NewValue(10),
		Gas:       1000,
		Input:     []byte{},
	}

	context.EXPECT().GetCodeHash(params.Recipient).Return(tosca.Hash{})
	context.EXPECT().GetCode(params.Recipient).Return([]byte{})
	context.EXPECT().CreateSnapshot()

	context.EXPECT().GetBalance(params.Sender).Return(tosca.NewValue(100)).Times(2)
	context.EXPECT().GetBalance(params.Recipient).Return(tosca.NewValue(0)).Times(2)
	context.EXPECT().SetBalance(params.Sender, tosca.NewValue(90))
	context.EXPECT().SetBalance(params.Recipient, tosca.NewValue(10))

	interpreter.EXPECT().Run(gomock.Any()).Return(tosca.Result{Success: true}, nil)

	_, err := runContext.Call(tosca.Call, params)
	if err != nil {
		t.Errorf("transferValue returned an error: %v", err)
	}
}

func TestCall_TransferValueInCreate(t *testing.T) {
	ctrl := gomock.NewController(t)
	context := tosca.NewMockTransactionContext(ctrl)
	interpreter := tosca.NewMockInterpreter(ctrl)
	runContext := runContext{
		context,
		interpreter,
		tosca.BlockParameters{},
		tosca.TransactionParameters{},
		0,
		false,
	}

	params := tosca.CallParameters{
		Sender: tosca.Address{1},
		Value:  tosca.NewValue(10),
		Gas:    1000,
		Input:  []byte{},
	}
	code := tosca.Code{}
	createdAddress := tosca.Address(crypto.CreateAddress(common.Address(params.Sender), 0))

	context.EXPECT().GetBalance(params.Sender).Return(tosca.NewValue(100))
	context.EXPECT().GetBalance(params.Recipient).Return(tosca.NewValue(0))
	context.EXPECT().GetNonce(params.Sender).Return(uint64(0))
	context.EXPECT().SetNonce(params.Sender, uint64(1))
	context.EXPECT().GetNonce((params.Sender)).Return(uint64(1))
	context.EXPECT().GetNonce(createdAddress).Return(uint64(0))
	context.EXPECT().HasEmptyStorage(createdAddress).Return(true)
	context.EXPECT().GetCodeHash(createdAddress).Return(tosca.Hash{})
	context.EXPECT().CreateSnapshot()
	context.EXPECT().CreateAccount(createdAddress)
	context.EXPECT().SetNonce(createdAddress, uint64(1))
	context.EXPECT().GetBalance(params.Sender).Return(tosca.NewValue(100))
	context.EXPECT().GetBalance(createdAddress).Return(tosca.NewValue(0))
	context.EXPECT().SetBalance(params.Sender, tosca.NewValue(90))
	context.EXPECT().SetBalance(createdAddress, tosca.NewValue(10))
	context.EXPECT().SetCode(createdAddress, code)

	interpreter.EXPECT().Run(gomock.Any()).Return(tosca.Result{Success: true, Output: tosca.Data(code)}, nil)

	result, err := runContext.Call(tosca.Create, params)
	if err != nil {
		t.Errorf("transferValue returned an error: %v", err)
	}
	if !result.Success {
		t.Errorf("transferValue was not successful")
	}
}

func TestTransferValue_InCallRestoreFailed(t *testing.T) {
	ctrl := gomock.NewController(t)
	context := tosca.NewMockTransactionContext(ctrl)
	interpreter := tosca.NewMockInterpreter(ctrl)
	runContext := runContext{
		context,
		interpreter,
		tosca.BlockParameters{},
		tosca.TransactionParameters{},
		0,
		false,
	}

	params := tosca.CallParameters{
		Sender:    tosca.Address{1},
		Recipient: tosca.Address{2},
		Value:     tosca.NewValue(10),
		Gas:       1000,
		Input:     []byte{},
	}
	context.EXPECT().GetBalance(params.Sender).Return(tosca.NewValue(0))

	result, err := runContext.Call(tosca.Call, params)
	if err != nil {
		t.Errorf("Correct execution of the transaction should not return an error")
	}

	if result.Success {
		t.Errorf("The transaction should have failed")
	}
}

func TestTransferValue_SuccessfulValueTransfer(t *testing.T) {
	values := map[string]tosca.Value{
		"zeroValue":     tosca.NewValue(0),
		"smallValue":    tosca.NewValue(10),
		"senderBalance": tosca.NewValue(100),
	}

	senderBalance := tosca.NewValue(100)
	recipientBalance := tosca.NewValue(0)

	ctrl := gomock.NewController(t)
	context := tosca.NewMockTransactionContext(ctrl)

	for name, value := range values {
		t.Run(name, func(t *testing.T) {
			transaction := tosca.Transaction{
				Sender:    tosca.Address{1},
				Recipient: &tosca.Address{2},
				Value:     value,
			}

			if name != "zeroValue" {
				context.EXPECT().GetBalance(transaction.Sender).Return(senderBalance)
				context.EXPECT().GetBalance(*transaction.Recipient).Return(recipientBalance)
			}

			if !canTransferValue(context, transaction.Value, transaction.Sender, transaction.Recipient) {
				t.Errorf("Value should be possible but was not")
			}
		})
	}
}

func TestTransferValue_FailedValueTransfer(t *testing.T) {
	transfers := map[string]struct {
		value           tosca.Value
		senderBalance   tosca.Value
		receiverBalance tosca.Value
	}{
		"insufficientBalance": {
			tosca.NewValue(100),
			tosca.NewValue(50),
			tosca.NewValue(0),
		},
		"overflow": {
			tosca.NewValue(100),
			tosca.NewValue(1000),
			tosca.NewValue(math.MaxUint64, math.MaxUint64, math.MaxUint64, math.MaxUint64-10),
		},
	}

	for name, transfer := range transfers {
		t.Run(name, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			context := tosca.NewMockTransactionContext(ctrl)

			context.EXPECT().GetBalance(tosca.Address{1}).Return(transfer.senderBalance).AnyTimes()
			context.EXPECT().GetBalance(tosca.Address{2}).Return(transfer.receiverBalance).AnyTimes()

			if canTransferValue(context, transfer.value, tosca.Address{1}, &tosca.Address{2}) {
				t.Errorf("value transfer should have returned an error")
			}
		})
	}
}

func TestCanTransferValue_SameSenderAndReceiver(t *testing.T) {
	tests := map[string]struct {
		value         tosca.Value
		expectedError bool
	}{
		"sufficientBalance":   {tosca.NewValue(10), false},
		"insufficientBalance": {tosca.NewValue(1000), true},
	}

	for _, test := range tests {
		ctrl := gomock.NewController(t)
		context := tosca.NewMockTransactionContext(ctrl)
		context.EXPECT().GetBalance(gomock.Any()).Return(tosca.NewValue(100))

		canTransfer := canTransferValue(context, test.value, tosca.Address{1}, &tosca.Address{1})
		if test.expectedError {
			if canTransfer {
				t.Errorf("transfer value should have not been possible")
			}
		} else {
			if !canTransfer {
				t.Errorf("transfer value should have been possible")
			}
		}
	}
}

func TestTransferValue_BalanceIsNotChangedWhenValueIsTransferredToTheSameAccount(t *testing.T) {
	ctrl := gomock.NewController(t)
	context := tosca.NewMockTransactionContext(ctrl)

	address := tosca.Address{1}
	value := tosca.NewValue(10)

	transferValue(context, value, address, address)
}

func TestCreate_CreateAddress_ProducesTheCorrectAddress(t *testing.T) {
	tests := map[string]struct {
		kind     tosca.CallKind
		sender   tosca.Address
		nonce    uint64
		salt     tosca.Hash
		initHash tosca.Hash
	}{
		"create": {
			kind:     tosca.Create,
			sender:   tosca.Address{1},
			nonce:    42,
			salt:     tosca.Hash{},
			initHash: tosca.Hash{},
		},
		"create2": {
			kind:     tosca.Create2,
			sender:   tosca.Address{1},
			nonce:    0,
			salt:     tosca.Hash{16, 32, 64},
			initHash: tosca.Hash{0x01, 0x02, 0x03, 0x04, 0x05},
		},
	}

	for name, test := range tests {
		for _, revision := range tosca.GetAllKnownRevisions() {
			t.Run(fmt.Sprintf("%s/%s", name, revision), func(t *testing.T) {
				var want tosca.Address

				switch test.kind {
				case tosca.Create:
					want = tosca.Address(crypto.CreateAddress(common.Address(test.sender), test.nonce))
				case tosca.Create2:
					initHash := crypto.Keccak256(test.initHash[:])
					want = tosca.Address(crypto.CreateAddress2(common.Address(test.sender), common.Hash(test.salt), initHash[:]))
				default:
					t.Fatalf("invalid call kind for create: %v", test.kind)
				}

				ctrl := gomock.NewController(t)
				context := tosca.NewMockTransactionContext(ctrl)
				// the sender nonce is already updated before the createAddress function.
				context.EXPECT().GetNonce(test.sender).Return(test.nonce + 1).AnyTimes()
				if revision > tosca.R07_Istanbul {
					context.EXPECT().AccessAccount(want)
				}
				context.EXPECT().GetNonce(want)
				context.EXPECT().HasEmptyStorage(want).Return(true)
				context.EXPECT().GetCodeHash(want)

				parameters := tosca.CallParameters{
					Sender: test.sender,
					Salt:   test.salt,
					Input:  test.initHash[:],
				}

				result, err := createAddress(test.kind, parameters, revision, context)
				if err != nil {
					t.Errorf("Unexpected error: %v", err)
				}
				if result != want {
					t.Errorf("Unexpected address, got: %v, want: %v", result, want)
				}
			})
		}
	}
}

func TestCreate_CreateAddress_UnsupportedKindTriggersError(t *testing.T) {
	ctrl := gomock.NewController(t)
	context := tosca.NewMockTransactionContext(ctrl)

	_, err := createAddress(tosca.Call, tosca.CallParameters{}, tosca.R07_Istanbul, context)
	require.ErrorContains(t, err, "invalid call kind for create")
}

func TestCreate_CreateAddressReturnErrorIfAddressIsNotEmpty(t *testing.T) {
	tests := map[string]struct {
		nonce         uint64
		emptyStorage  bool
		codeHash      tosca.Hash
		expectedError error
	}{
		"nonce": {
			nonce:         1,
			emptyStorage:  true,
			codeHash:      tosca.Hash{},
			expectedError: fmt.Errorf("created address is not empty"),
		},
		"emptyStorage": {
			nonce:         0,
			emptyStorage:  false,
			codeHash:      tosca.Hash{},
			expectedError: fmt.Errorf("created address is not empty"),
		},
		"codeHash": {
			nonce:         0,
			emptyStorage:  true,
			codeHash:      tosca.Hash{1},
			expectedError: fmt.Errorf("created address is not empty"),
		},
		"emptyCodeHash": {
			nonce:         0,
			emptyStorage:  true,
			codeHash:      tosca.Hash{},
			expectedError: nil,
		},
		"zeroCodeHash": {
			nonce:         0,
			emptyStorage:  true,
			codeHash:      tosca.Hash{},
			expectedError: nil,
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			context := tosca.NewMockTransactionContext(ctrl)
			context.EXPECT().GetNonce(gomock.Any()).Return(test.nonce).MinTimes(1)
			context.EXPECT().HasEmptyStorage(gomock.Any()).Return(test.emptyStorage).AnyTimes()
			context.EXPECT().GetCodeHash(gomock.Any()).Return(test.codeHash).AnyTimes()

			_, err := createAddress(tosca.Create, tosca.CallParameters{}, tosca.R07_Istanbul, context)
			if test.expectedError != nil {
				require.ErrorContains(t, err, test.expectedError.Error())
			} else {
				require.NoError(t, err)
			}
		})
	}
}

func TestCreate_CheckAndDeployCode_SetsCodeOrResetsResult(t *testing.T) {
	tests := map[string]struct {
		revision      tosca.Revision
		code          []byte
		gas           tosca.Gas
		success       bool
		resultGasLeft tosca.Gas
	}{
		"successful": {
			revision:      tosca.R07_Istanbul,
			code:          []byte{1, 2, 3},
			gas:           tosca.Gas(601),
			success:       true,
			resultGasLeft: tosca.Gas(1),
		},
		"greater max code size": {
			revision:      tosca.R07_Istanbul,
			code:          make([]byte, maxCodeSize+1),
			gas:           tosca.Gas(100000000),
			success:       false,
			resultGasLeft: tosca.Gas(0),
		},
		"starts with 0xEF pre Shanghai": {
			revision:      tosca.R07_Istanbul,
			code:          append([]byte{0xEF}, []byte{1, 2, 3}...),
			gas:           tosca.Gas(801),
			success:       true,
			resultGasLeft: tosca.Gas(1),
		},
		"starts with 0xEF Shanghai": {
			revision:      tosca.R12_Shanghai,
			code:          append([]byte{0xEF}, []byte{1, 2, 3}...),
			gas:           tosca.Gas(801),
			success:       false,
			resultGasLeft: tosca.Gas(0),
		},
		"too little gas": {
			revision:      tosca.R07_Istanbul,
			code:          []byte{1, 2, 3},
			gas:           tosca.Gas(599),
			success:       false,
			resultGasLeft: tosca.Gas(0),
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			context := tosca.NewMockTransactionContext(ctrl)

			createdAddress := tosca.Address{1}
			result := tosca.Result{
				Success: true,
				Output:  test.code,
				GasLeft: test.gas,
			}

			if test.success {
				context.EXPECT().SetCode(createdAddress, tosca.Code(test.code))
			}

			finalizedResult := checkAndDeployCode(result, createdAddress, test.revision, context)

			result.GasLeft = test.resultGasLeft
			if !test.success {
				result.Output = nil
				result.Success = false
			}
			require.Equal(t, result, finalizedResult, "Finalized result does not match expected result")
		})
	}
}

func TestCreate_senderCreateSetUp_ReturnsError(t *testing.T) {
	tests := map[string]struct {
		nonce uint64
		value tosca.Value
		err   error
	}{
		"nonce overflow": {
			nonce: math.MaxUint64,
			err:   fmt.Errorf("nonce overflow"),
		},
		"insufficient balance": {
			nonce: 0,
			value: tosca.NewValue(1000),
			err:   fmt.Errorf("insufficient balance"),
		},
		"successful": {
			nonce: 0,
			value: tosca.NewValue(0),
			err:   nil,
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			context := tosca.NewMockTransactionContext(ctrl)

			sender := tosca.Address{1}
			context.EXPECT().GetBalance(sender).Return(tosca.NewValue(100)).AnyTimes()
			context.EXPECT().GetNonce(sender).Return(test.nonce).AnyTimes()
			context.EXPECT().SetNonce(sender, test.nonce+1).AnyTimes()

			parameters := tosca.CallParameters{
				Sender: sender,
				Value:  test.value,
			}

			err := senderCreateSetUp(parameters, context)
			if test.err != nil {
				require.ErrorContains(t, err, test.err.Error(), "Expected error did not match")
			} else {
				require.NoError(t, err, "Expected no error but got one")
			}
		})
	}
}

func TestIncrementNonce(t *testing.T) {
	tests := map[string]struct {
		nonce uint64
		err   error
	}{
		"zero": {
			nonce: 0,
			err:   nil,
		},
		"max": {
			nonce: math.MaxUint64,
			err:   fmt.Errorf("nonce overflow"),
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			context := tosca.NewMockTransactionContext(ctrl)
			context.EXPECT().GetNonce(gomock.Any()).Return(test.nonce)
			context.EXPECT().SetNonce(gomock.Any(), test.nonce+1).AnyTimes()

			err := incrementNonce(context, tosca.Address{})
			if test.err != nil && err == nil {
				t.Errorf("incrementNonce returned an unexpected error: %v", err)
			}
		})
	}
}

func TestRunContext_AccountIsOnlyCreatedIfItIsEmptyAndDoesNotExist(t *testing.T) {
	tests := map[string]struct {
		nonce        uint64
		emptyStorage bool
		codeHash     tosca.Hash
		exists       bool
		successful   bool
	}{
		"non empty with nonce": {
			nonce:        1,
			emptyStorage: true,
			codeHash:     tosca.Hash{},
			successful:   false,
		},
		"non empty with storage": {
			nonce:        0,
			emptyStorage: false,
			codeHash:     tosca.Hash{},
			successful:   false,
		},
		"non empty with code": {
			nonce:        0,
			emptyStorage: true,
			codeHash:     tosca.Hash{1, 2, 3},
			successful:   false,
		},
		"empty but exists": {
			nonce:        0,
			emptyStorage: true,
			codeHash:     tosca.Hash{},
			exists:       true,
			successful:   true,
		},
		"empty and does not exist": {
			nonce:        0,
			emptyStorage: true,
			codeHash:     tosca.Hash{},
			exists:       false,
			successful:   true,
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			context := tosca.NewMockTransactionContext(ctrl)
			interpreter := tosca.NewMockInterpreter(ctrl)
			runContext := runContext{
				context,
				interpreter,
				tosca.BlockParameters{},
				tosca.TransactionParameters{},
				0,
				false,
			}

			params := tosca.CallParameters{
				Sender: tosca.Address{1},
				Gas:    1000,
				Input:  []byte{},
			}
			code := tosca.Code{}
			senderNonce := uint64(1)
			createdAddress := tosca.Address(crypto.CreateAddress(common.Address(params.Sender), senderNonce))

			context.EXPECT().GetNonce(params.Sender).Return(senderNonce)
			context.EXPECT().SetNonce(params.Sender, senderNonce+1)
			context.EXPECT().GetNonce((params.Sender)).Return(senderNonce + 1)

			context.EXPECT().GetNonce(createdAddress).Return(test.nonce)
			context.EXPECT().HasEmptyStorage(createdAddress).Return(test.emptyStorage).AnyTimes()
			context.EXPECT().GetCodeHash(createdAddress).Return(test.codeHash).AnyTimes()
			context.EXPECT().CreateSnapshot().AnyTimes()
			context.EXPECT().AccountExists(createdAddress).Return(test.exists).AnyTimes()
			context.EXPECT().CreateAccount(createdAddress).AnyTimes()

			context.EXPECT().SetNonce(createdAddress, uint64(1)).AnyTimes()
			context.EXPECT().SetCode(createdAddress, code).AnyTimes()

			interpreter.EXPECT().Run(gomock.Any()).Return(tosca.Result{Success: true, Output: tosca.Data(code)}, nil).AnyTimes()

			result, err := runContext.Call(tosca.Create, params)
			if err != nil {
				t.Errorf("transferValue returned an error: %v", err)
			}
			if result.Success != test.successful {
				t.Errorf("transferValue was not successful")
			}
		})
	}
}

func TestRunContext_runInterpreterSelectsCodeBasedOnType(t *testing.T) {
	code := tosca.Code{1, 2, 3}
	codeHash := tosca.Hash{4, 5, 6}
	codeAddress := tosca.Address{1}
	recipient := tosca.Address{2}

	tests := map[string]struct {
		kind      tosca.CallKind
		mockSetup func(context *tosca.MockTransactionContext)
	}{
		"call": {
			kind: tosca.Call,
			mockSetup: func(context *tosca.MockTransactionContext) {
				context.EXPECT().GetCodeHash(recipient).Return(codeHash)
				context.EXPECT().GetCode(recipient).Return(code)
			},
		},
		"staticCall": {
			kind: tosca.StaticCall,
			mockSetup: func(context *tosca.MockTransactionContext) {
				context.EXPECT().GetCodeHash(recipient).Return(codeHash)
				context.EXPECT().GetCode(recipient).Return(code)
			},
		},
		"delegateCall": {
			kind: tosca.DelegateCall,
			mockSetup: func(context *tosca.MockTransactionContext) {
				context.EXPECT().GetCodeHash(codeAddress).Return(codeHash)
				context.EXPECT().GetCode(codeAddress).Return(code)
			},
		},
		"codeCall": {
			kind: tosca.CallCode,
			mockSetup: func(context *tosca.MockTransactionContext) {
				context.EXPECT().GetCodeHash(codeAddress).Return(codeHash)
				context.EXPECT().GetCode(codeAddress).Return(code)
			},
		},
		"create": {
			kind: tosca.Create,
			mockSetup: func(context *tosca.MockTransactionContext) {
				// no calls to state DB
			},
		},
		"create2": {
			kind: tosca.Create2,
			mockSetup: func(context *tosca.MockTransactionContext) {
				// no calls to state DB
			},
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			context := tosca.NewMockTransactionContext(ctrl)
			interpreter := tosca.NewMockInterpreter(ctrl)
			runContext := runContext{
				context,
				interpreter,
				tosca.BlockParameters{},
				tosca.TransactionParameters{},
				0,
				false,
			}

			parameters := tosca.CallParameters{
				Sender:      tosca.Address{1},
				Recipient:   recipient,
				CodeAddress: codeAddress,
				Gas:         1000,
				Input:       tosca.Data(code),
			}

			test.mockSetup(context)

			interpreter.EXPECT().Run(gomock.Any()).DoAndReturn(func(parameters tosca.Parameters) (tosca.Result, error) {
				if test.kind == tosca.Create || test.kind == tosca.Create2 {
					require.Nil(t, parameters.Input)
					require.Equal(t, parameters.Code, code)
					require.NotNil(t, parameters.CodeHash)
				} else {
					require.Equal(t, parameters.Code, code)
					require.Equal(t, *parameters.CodeHash, codeHash)
				}
				return tosca.Result{Success: true}, nil
			})

			_, err := runContext.runInterpreter(test.kind, parameters)
			require.NoError(t, err)
		})
	}
}

func TestRunContext_runInterpreterCreateComputesCorrectCodeHash(t *testing.T) {
	code := tosca.Code{1, 2, 3}
	expectedHash := tosca.Hash(crypto.Keccak256(code))

	ctrl := gomock.NewController(t)
	interpreter := tosca.NewMockInterpreter(ctrl)
	runContext := runContext{
		nil,
		interpreter,
		tosca.BlockParameters{},
		tosca.TransactionParameters{},
		0,
		false,
	}

	interpreter.EXPECT().Run(gomock.Any()).DoAndReturn(func(parameters tosca.Parameters) (tosca.Result, error) {
		require.Nil(t, parameters.Input)
		require.Equal(t, parameters.Code, code)
		require.Equal(t, *parameters.CodeHash, expectedHash)
		return tosca.Result{Success: true}, nil
	})

	_, err := runContext.runInterpreter(tosca.Create, tosca.CallParameters{
		Input: tosca.Data(code),
	})
	require.NoError(t, err)
}

func TestRunContext_runInterpreterForwardsValuesCorrectly(t *testing.T) {
	ctrl := gomock.NewController(t)
	context := tosca.NewMockTransactionContext(ctrl)
	interpreter := tosca.NewMockInterpreter(ctrl)
	runContext := runContext{
		context,
		interpreter,
		tosca.BlockParameters{
			ChainID: tosca.Word{0x01},
		},
		tosca.TransactionParameters{
			Origin: tosca.Address{0x02},
		},
		0,
		false,
	}

	parameters := tosca.CallParameters{
		Sender:      tosca.Address{0x03},
		Recipient:   tosca.Address{0x04},
		CodeAddress: tosca.Address{0x05},
		Gas:         1000,
		Input:       []byte("test input"),
		Value:       tosca.NewValue(42),
	}

	code := tosca.Code{1, 2, 3}
	codeHash := tosca.Hash{4, 5, 6}

	expectedParams := tosca.Parameters{
		BlockParameters:       runContext.blockParameters,
		TransactionParameters: runContext.transactionParameters,
		Context:               runContext,
		Sender:                parameters.Sender,
		Recipient:             parameters.Recipient,
		Gas:                   parameters.Gas,
		Input:                 parameters.Input,
		Value:                 parameters.Value,
		Code:                  code,
		CodeHash:              &codeHash,
	}

	context.EXPECT().GetCode(parameters.Recipient).Return(code)
	context.EXPECT().GetCodeHash(parameters.Recipient).Return(codeHash)

	interpreter.EXPECT().Run(gomock.Any()).DoAndReturn(func(p tosca.Parameters) (tosca.Result, error) {
		require.Equal(t, p.Depth, expectedParams.Depth-1)
		require.Equal(t, p.BlockParameters, expectedParams.BlockParameters)
		require.Equal(t, p.TransactionParameters, expectedParams.TransactionParameters)
		require.Equal(t, p.Context, expectedParams.Context)
		require.Equal(t, p.Sender, expectedParams.Sender)
		require.Equal(t, p.Recipient, expectedParams.Recipient)
		require.Equal(t, p.Gas, expectedParams.Gas)
		require.Equal(t, p.Input, expectedParams.Input)
		require.Equal(t, p.Value, expectedParams.Value)
		require.Equal(t, p.Code, expectedParams.Code)
		require.Equal(t, *p.CodeHash, *expectedParams.CodeHash)
		return tosca.Result{Success: true}, nil
	})

	_, err := runContext.runInterpreter(tosca.Call, parameters)
	require.NoError(t, err)
}

func TestCall_PrecompiledCheckDependsOnCodeAddress(t *testing.T) {
	tests := map[string]struct {
		codeAddress tosca.Address
	}{
		"precompiled":   {test_utils.NewAddress(0x01)},
		"stateContract": {StateContractAddress()},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			context := tosca.NewMockTransactionContext(ctrl)
			context.EXPECT().CreateSnapshot()

			// No calls to the interpreter because the call is handled by the precompiled contract.
			interpreter := tosca.NewMockInterpreter(ctrl)

			runContext := runContext{
				context,
				interpreter,
				tosca.BlockParameters{},
				tosca.TransactionParameters{},
				0,
				false,
			}

			input := []byte{}
			if name == "stateContract" {
				// Use set balance method of state contract to create a successful call.
				input = append([]byte{0xe3, 0x4, 0x43, 0xbc}, make([]byte, 64)...)
				context.EXPECT().SetBalance(tosca.Address{}, tosca.Value{})
			}

			result, err := runContext.executeCall(tosca.Call, tosca.CallParameters{
				Sender:      DriverAddress(),
				Recipient:   tosca.Address{2},
				CodeAddress: test.codeAddress,
				Value:       tosca.NewValue(0),
				Gas:         100000,
				Input:       input,
			})

			if err != nil {
				t.Errorf("unexpected error: %v", err)
			}
			if !result.Success {
				t.Error("expected successful call, got failure")
			}
		})
	}
}

func TestRunContext_InterpreterErrorIsForwardedAndSnapshotIsRestored(t *testing.T) {
	calls := allCallTypes()
	for _, call := range calls {
		t.Run(call.String(), func(t *testing.T) {
			ctrl := gomock.NewController(t)
			context := tosca.NewMockTransactionContext(ctrl)
			interpreter := tosca.NewMockInterpreter(ctrl)
			runContext := runContext{
				TransactionContext: context,
				interpreter:        interpreter,
			}

			parameters := tosca.CallParameters{
				Sender:      tosca.Address{1},
				Recipient:   tosca.Address{2},
				CodeAddress: tosca.Address{2},
				Value:       tosca.NewValue(0),
				Gas:         1000,
				Input:       []byte{},
			}

			snapshot := tosca.Snapshot(42)

			context.EXPECT().CreateSnapshot().Return(snapshot)
			if call == tosca.Create || call == tosca.Create2 {
				context.EXPECT().GetNonce(parameters.Sender)
				context.EXPECT().SetNonce(parameters.Sender, uint64(1))
				context.EXPECT().GetNonce(gomock.Any()).AnyTimes()
				context.EXPECT().HasEmptyStorage(gomock.Any()).Return(true)
				context.EXPECT().GetCodeHash(gomock.Any())
				context.EXPECT().CreateAccount(gomock.Any())
				context.EXPECT().SetNonce(gomock.Any(), uint64(1))
			} else {
				context.EXPECT().GetCode(parameters.Recipient)
				context.EXPECT().GetCodeHash(parameters.Recipient)
			}
			// Make sure the correct snapshot is rolled back
			context.EXPECT().RestoreSnapshot(snapshot)

			interpreterError := fmt.Errorf("interpreter error")
			interpreter.EXPECT().Run(gomock.Any()).Return(tosca.Result{}, interpreterError)

			result, err := runContext.Call(call, parameters)
			require.Equal(t, interpreterError, err)
			require.Equal(t, tosca.CallResult{}, result)
		})
	}
}

func TestRunContext_UnsuccessfulInterpreterExecutionRestoresSnapshot(t *testing.T) {
	for _, call := range allCallTypes() {
		t.Run(call.String(), func(t *testing.T) {
			ctrl := gomock.NewController(t)
			context := tosca.NewMockTransactionContext(ctrl)
			interpreter := tosca.NewMockInterpreter(ctrl)
			runContext := runContext{
				TransactionContext: context,
				interpreter:        interpreter,
			}

			parameters := tosca.CallParameters{
				Sender:      tosca.Address{1},
				Recipient:   tosca.Address{2},
				CodeAddress: tosca.Address{2},
				Value:       tosca.NewValue(0),
				Gas:         1000,
				Input:       []byte{},
			}

			snapshot := tosca.Snapshot(42)

			context.EXPECT().CreateSnapshot().Return(snapshot)
			if call == tosca.Create || call == tosca.Create2 {
				context.EXPECT().GetNonce(parameters.Sender)
				context.EXPECT().SetNonce(parameters.Sender, uint64(1))
				context.EXPECT().GetNonce(gomock.Any()).AnyTimes()
				context.EXPECT().HasEmptyStorage(gomock.Any()).Return(true)
				context.EXPECT().GetCodeHash(gomock.Any())
				context.EXPECT().CreateAccount(gomock.Any())
				context.EXPECT().SetNonce(gomock.Any(), uint64(1))
			} else {
				context.EXPECT().GetCode(parameters.Recipient)
				context.EXPECT().GetCodeHash(parameters.Recipient)
			}
			// Make sure the correct snapshot is rolled back
			context.EXPECT().RestoreSnapshot(snapshot)

			output := tosca.Data("some output")
			gasLeft := tosca.Gas(500)
			gasRefund := tosca.Gas(100)
			interpreter.EXPECT().Run(gomock.Any()).Return(
				tosca.Result{
					GasLeft:   gasLeft,
					GasRefund: gasRefund,
					Output:    output,
				},
				nil,
			)

			result, err := runContext.Call(call, parameters)
			require.NoError(t, err)
			require.False(t, result.Success)
			require.Equal(t, output, result.Output)
			require.Equal(t, gasLeft, result.GasLeft)
			require.Equal(t, gasRefund, result.GasRefund)
			if call == tosca.Create || call == tosca.Create2 {
				require.NotEqual(t, tosca.Address{}, result.CreatedAddress)
			}
		})
	}
}

func allCallTypes() []tosca.CallKind {
	return []tosca.CallKind{
		tosca.Call,
		tosca.StaticCall,
		tosca.CallCode,
		tosca.DelegateCall,
		tosca.Create,
		tosca.Create2,
	}
}
