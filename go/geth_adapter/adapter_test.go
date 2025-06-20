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
	"bytes"
	"errors"
	"fmt"
	"math"
	"math/big"
	"reflect"
	"slices"
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/tracing"
	geth "github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/params"
	"github.com/holiman/uint256"
	"github.com/stretchr/testify/require"
	"go.uber.org/mock/gomock"
)

//go:generate mockgen -source adapter_test.go -destination adapter_test_mocks.go -package geth_adapter

type CallContextInterceptor interface {
	geth.CallContextInterceptor
}

type StateDb interface {
	geth.StateDB
}

func TestGethAdapter_RunContextAdapterImplementsRunContextInterface(t *testing.T) {
	var _ tosca.RunContext = &runContextAdapter{}
}

func TestRunContextAdapter_SetBalanceHasCorrectEffect(t *testing.T) {
	tests := []struct {
		before tosca.Value
		after  tosca.Value
		add    tosca.Value
		sub    tosca.Value
	}{
		{},
		{
			before: tosca.NewValue(10),
			after:  tosca.NewValue(10),
		},
		{
			before: tosca.NewValue(0),
			after:  tosca.NewValue(1),
			add:    tosca.NewValue(1),
		},
		{
			before: tosca.NewValue(1),
			after:  tosca.NewValue(0),
			sub:    tosca.NewValue(1),
		},
		{
			before: tosca.NewValue(123),
			after:  tosca.NewValue(321),
			add:    tosca.NewValue(321 - 123),
		},
	}

	for _, test := range tests {
		t.Run(fmt.Sprintf("%v_to_%v", test.before, test.after), func(t *testing.T) {
			ctrl := gomock.NewController(t)
			stateDb := NewMockStateDb(ctrl)
			stateDb.EXPECT().GetBalance(gomock.Any()).Return(test.before.ToUint256())
			if test.add != (tosca.Value{}) {
				diff := test.add.ToUint256()
				stateDb.EXPECT().AddBalance(gomock.Any(), diff, gomock.Any())
			}
			if test.sub != (tosca.Value{}) {
				diff := test.sub.ToUint256()
				stateDb.EXPECT().SubBalance(gomock.Any(), diff, gomock.Any())
			}

			adapter := &runContextAdapter{evm: &geth.EVM{StateDB: stateDb}}
			adapter.SetBalance(tosca.Address{}, test.after)
		})
	}
}

func TestRunContextAdapter_SetAndGetNonce(t *testing.T) {
	ctrl := gomock.NewController(t)
	stateDb := NewMockStateDb(ctrl)
	adapter := &runContextAdapter{evm: &geth.EVM{StateDB: stateDb}}

	address := tosca.Address{0x42}
	nonce := uint64(123)

	stateDb.EXPECT().SetNonce(common.Address(address), nonce, tracing.NonceChangeUnspecified)
	adapter.SetNonce(address, nonce)

	stateDb.EXPECT().GetNonce(common.Address(address)).Return(nonce)
	got := adapter.GetNonce(address)
	if got != nonce {
		t.Errorf("Got wrong nonce %v, expected %v", got, nonce)
	}
}

func TestRunContextAdapter_SetAndGetCode(t *testing.T) {
	ctrl := gomock.NewController(t)
	stateDb := NewMockStateDb(ctrl)
	adapter := &runContextAdapter{evm: &geth.EVM{StateDB: stateDb}}

	address := tosca.Address{0x42}
	code := []byte{1, 2, 3}

	stateDb.EXPECT().SetCode(common.Address(address), code)
	adapter.SetCode(address, code)

	stateDb.EXPECT().GetCode(common.Address(address)).Return(code)
	got := adapter.GetCode(address)
	if !bytes.Equal(got, code) {
		t.Errorf("Got wrong code %v, expected %v", got, code)
	}
}

func TestRunContextAdapter_SetAndGetStorage(t *testing.T) {
	ctrl := gomock.NewController(t)
	stateDb := NewMockStateDb(ctrl)
	adapter := &runContextAdapter{evm: &geth.EVM{StateDB: stateDb}}

	address := tosca.Address{0x42}
	key := tosca.Key{10}
	original := tosca.Word{0}
	current := tosca.Word{1}
	future := tosca.Word{2}

	stateDb.EXPECT().GetState(common.Address(address), common.Hash(key)).Return(common.Hash(current))
	stateDb.EXPECT().GetCommittedState(common.Address(address), common.Hash(key)).Return(common.Hash(original))
	stateDb.EXPECT().SetState(common.Address(address), common.Hash(key), common.Hash(future))
	status := adapter.SetStorage(address, key, future)
	if status != tosca.StorageAssigned {
		t.Errorf("Storage status did not match expected, want %v, got %v", tosca.StorageAssigned, status)
	}

	stateDb.EXPECT().GetState(common.Address(address), common.Hash(key)).Return(common.Hash(current))
	got := adapter.GetStorage(address, key)
	if got != current {
		t.Errorf("Got wrong storage value %v, expected %v", got, current)
	}
}

func TestRunContextAdapter_GetAndSetTransientStorage(t *testing.T) {
	ctrl := gomock.NewController(t)
	stateDb := NewMockStateDb(ctrl)
	adapter := &runContextAdapter{evm: &geth.EVM{StateDB: stateDb}}

	address := tosca.Address{0x42}
	key := tosca.Key{10}
	value := tosca.Word{100}

	stateDb.EXPECT().SetTransientState(common.Address(address), common.Hash(key), common.Hash(value))
	adapter.SetTransientStorage(address, key, value)

	stateDb.EXPECT().GetTransientState(common.Address(address), common.Hash(key)).Return(common.Hash(value))
	got := adapter.GetTransientStorage(address, key)
	if got != value {
		t.Errorf("Got wrong transient storage value %v, expected %v", got, value)
	}
}

func TestRunContextAdapter_SelfDestruct(t *testing.T) {
	cancunTime := uint64(42)
	londonBlock := big.NewInt(42)
	tests := map[string]struct {
		selfdestructed bool
		blockTime      uint64
	}{
		"selfdestructedPreCancun": {
			true,
			cancunTime - 1,
		},
		"notSelfdestructedPreCancun": {
			false,
			cancunTime - 1,
		},
		"selddestructedCancun": {
			true,
			cancunTime,
		},
		"notSelfdestructedCancun": {
			false,
			cancunTime,
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			stateDb := NewMockStateDb(ctrl)

			address := common.Address{0x42}
			beneficiary := common.Address{0x43}

			blockContext := geth.BlockContext{
				BlockNumber: londonBlock.Add(londonBlock, big.NewInt(1)),
				Time:        test.blockTime,
			}
			chainConfig := &params.ChainConfig{
				CancunTime:  &cancunTime,
				LondonBlock: londonBlock,
				ChainID:     big.NewInt(42),
			}
			evm := geth.NewEVM(blockContext,
				stateDb,
				chainConfig,
				geth.Config{},
			)
			adapter := &runContextAdapter{evm: evm, caller: address}

			stateDb.EXPECT().HasSelfDestructed(address).Return(test.selfdestructed)
			stateDb.EXPECT().GetBalance(address).Return(uint256.NewInt(42))
			stateDb.EXPECT().AddBalance(common.Address(beneficiary), uint256.NewInt(42), tracing.BalanceDecreaseSelfdestruct)

			if test.blockTime < cancunTime {
				stateDb.EXPECT().SelfDestruct(address)
			} else {
				stateDb.EXPECT().SubBalance(address, uint256.NewInt(42), tracing.BalanceDecreaseSelfdestruct)
				stateDb.EXPECT().SelfDestruct6780(address)
			}

			got := adapter.SelfDestruct(tosca.Address(address), tosca.Address(beneficiary))
			if got == test.selfdestructed {
				t.Errorf("Selfdestruct should only return true if it has not been called before")
			}
		})
	}
}

func TestRunContextAdapter_SnapshotHandling(t *testing.T) {
	ctrl := gomock.NewController(t)
	stateDb := NewMockStateDb(ctrl)
	adapter := &runContextAdapter{evm: &geth.EVM{StateDB: stateDb}}

	snapshot := tosca.Snapshot(1)

	stateDb.EXPECT().RevertToSnapshot(int(snapshot))
	adapter.RestoreSnapshot(snapshot)

	stateDb.EXPECT().Snapshot().Return(int(snapshot))
	got := adapter.CreateSnapshot()
	if got != snapshot {
		t.Errorf("Got wrong snapshot %v, expected %v", got, snapshot)
	}
}

func TestRunContextAdapter_AccountOperations(t *testing.T) {
	ctrl := gomock.NewController(t)
	stateDb := NewMockStateDb(ctrl)
	adapter := &runContextAdapter{evm: &geth.EVM{StateDB: stateDb}}

	address := tosca.Address{0x42}

	stateDb.EXPECT().AddressInAccessList(common.Address(address)).Return(false)
	stateDb.EXPECT().AddAddressToAccessList(common.Address(address))
	accessStatus := adapter.AccessAccount(address)
	if accessStatus != tosca.ColdAccess {
		t.Errorf("Got wrong access type %v, expected %v", accessStatus, tosca.ColdAccess)
	}

	stateDb.EXPECT().Exist(common.Address(address)).Return(true)
	exits := adapter.AccountExists(address)
	if !exits {
		t.Errorf("Account should exist")
	}

	// Ensure that both CreateAccount and CreateContract are called when the account does not exist
	stateDb.EXPECT().Exist(common.Address(address)).Return(false)
	stateDb.EXPECT().CreateAccount(common.Address(address))
	stateDb.EXPECT().CreateContract(common.Address(address))
	adapter.CreateAccount(address)

	stateDb.EXPECT().AddressInAccessList(common.Address(address)).Return(true)
	inAccessList := adapter.IsAddressInAccessList(address)
	if !inAccessList {
		t.Errorf("Address should be in access list")
	}
}

func TestRunContextAdapter_Call(t *testing.T) {
	ctrl := gomock.NewController(t)
	stateDb := NewMockStateDb(ctrl)

	address := common.Address{0x42}

	stateDb.EXPECT().Snapshot().Return(1)
	stateDb.EXPECT().Exist(address).Return(true)
	stateDb.EXPECT().GetCode(address).Return([]byte{})

	canTransfer := func(geth.StateDB, common.Address, *uint256.Int) bool { return true }
	transfer := func(geth.StateDB, common.Address, common.Address, *uint256.Int) {}

	chainConfig := &params.ChainConfig{
		ChainID:       big.NewInt(42),
		IstanbulBlock: big.NewInt(24),
	}
	blockContext := geth.BlockContext{
		CanTransfer: canTransfer,
		Transfer:    transfer,
		BlockNumber: big.NewInt(24),
	}
	evm := geth.NewEVM(blockContext, stateDb, chainConfig, geth.Config{})

	runContextAdapter := &runContextAdapter{
		evm:    evm,
		caller: address,
	}

	gas := tosca.Gas(42)

	parameters := tosca.CallParameters{
		Recipient: tosca.Address(address),
		Gas:       gas,
	}

	result, err := runContextAdapter.Call(tosca.Call, parameters)
	if err != nil {
		t.Errorf("Unexpected error: %v", err)
	}
	if !result.Success {
		t.Errorf("Call was not successful")
	}
	if result.GasLeft != gas {
		t.Errorf("Call has the wrong amount of gas left: %v, expected: %v", result.GasLeft, gas)
	}
}

func TestRunContextAdapter_Call_LeftGasIsConstraintByZeroAndInputGas(t *testing.T) {
	ctrl := gomock.NewController(t)
	calls := NewMockCallContextInterceptor(ctrl)
	for gasIn := tosca.Gas(-100); gasIn < tosca.Gas(100); gasIn++ {
		for gasOut := tosca.Gas(-100); gasOut < tosca.Gas(100); gasOut++ {
			any := gomock.Any()
			calls.EXPECT().Call(any, any, any, any, uint64(gasIn), any).Return(
				nil, uint64(gasOut), nil,
			)

			evm := newEVMWithPassingChainConfig()
			evm.CallInterceptor = calls
			adapter := &runContextAdapter{evm: evm}
			result, err := adapter.Call(tosca.Call, tosca.CallParameters{
				Gas: gasIn,
			})
			if err != nil {
				t.Fatalf("Unexpected error: %v", err)
			}

			want := max(0, min(gasIn, gasOut))
			if got := result.GasLeft; got != want {
				t.Fatalf("Gas left should be equal to %v, got %v", want, got)
			}
		}
	}
}

func TestRunContextAdapter_Call_LeftGasOverflowLeadsToZeroGas(t *testing.T) {
	ctrl := gomock.NewController(t)
	calls := NewMockCallContextInterceptor(ctrl)

	overflows := []uint64{
		math.MaxInt64 + 1,
		math.MaxInt64 + 2,
		math.MaxUint64 - 1,
		math.MaxUint64,
	}

	for _, gasOut := range overflows {
		any := gomock.Any()
		calls.EXPECT().Call(any, any, any, any, any, any).Return(
			nil, gasOut, nil,
		)

		evm := newEVMWithPassingChainConfig()
		evm.CallInterceptor = calls
		adapter := &runContextAdapter{evm: evm}
		result, err := adapter.Call(tosca.Call, tosca.CallParameters{
			Gas: 42,
		})
		if err != nil {
			t.Fatalf("Unexpected error: %v", err)
		}
		if want, got := tosca.Gas(0), result.GasLeft; want != got {
			t.Fatalf("Gas left should be equal to %v, got %v", want, got)
		}
	}
}

func TestRunContextAdapter_getPrevRandaoReturnsHashBasedOnRevision(t *testing.T) {
	tests := map[string]struct {
		revision tosca.Revision
		want     tosca.Hash
	}{
		"london": {
			revision: tosca.R10_London,
			want:     tosca.Hash(tosca.NewValue(42)),
		},
		"paris": {
			revision: tosca.R11_Paris,
			want:     tosca.Hash{0x24},
		},
		"shanghai": {
			revision: tosca.R12_Shanghai,
			want:     tosca.Hash{0x24},
		},
	}

	random := common.Hash{0x24}
	context := geth.BlockContext{
		Difficulty: big.NewInt(42),
		Random:     &random,
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {

			got, err := getPrevRandao(&context, test.revision)
			if err != nil {
				t.Errorf("Unexpected error: %v", err)
			}
			if got != test.want {
				t.Errorf("Got wrong prevRandao %v, expected %v", got, test.want)
			}
		})
	}
}

func TestRunContextAdapter_getPrevRandaoErrorIfDifficultyCanNotBeConverted(t *testing.T) {
	context := geth.BlockContext{
		Difficulty: big.NewInt(-42),
	}

	_, err := getPrevRandao(&context, tosca.R10_London)
	if err == nil {
		t.Errorf("Expected error, got nil")
	}
}

func TestRunContextAdapter_Run(t *testing.T) {
	tests := map[string]bool{
		"success": true,
		"failure": false,
	}

	for name, success := range tests {
		t.Run(name, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			stateDb := NewMockStateDb(ctrl)
			interpreter := tosca.NewMockInterpreter(ctrl)

			chainId := int64(42)
			blockNumber := int64(24)
			address := tosca.Address{0x42}

			blockParameters := geth.BlockContext{BlockNumber: big.NewInt(blockNumber)}
			chainConfig := &params.ChainConfig{ChainID: big.NewInt(chainId), IstanbulBlock: big.NewInt(23)}
			evm := geth.NewEVM(blockParameters, stateDb, chainConfig, geth.Config{})
			adapter := &gethInterpreterAdapter{
				evm:         evm,
				interpreter: interpreter,
			}

			blockParams := tosca.BlockParameters{
				ChainID:     tosca.Word(tosca.NewValue(uint64(chainId))),
				BlockNumber: blockNumber,
			}
			expectedParams := tosca.Parameters{
				BlockParameters: blockParams,
				Kind:            tosca.Call,
				Static:          false,
				Recipient:       address,
				Sender:          address,
			}

			interpreter.EXPECT().Run(gomock.Any()).DoAndReturn(func(params tosca.Parameters) (tosca.Result, error) {
				// The parameters save the context as a pointer, its value can
				// not be predicted during the setup phase of the mock.
				if expectedParams.ChainID != params.ChainID ||
					expectedParams.BlockNumber != params.BlockNumber ||
					expectedParams.Timestamp != params.Timestamp ||
					expectedParams.Coinbase != params.Coinbase ||
					expectedParams.GasLimit != params.GasLimit ||
					expectedParams.PrevRandao != params.PrevRandao ||
					expectedParams.BaseFee != params.BaseFee ||
					expectedParams.BlobBaseFee != params.BlobBaseFee ||
					expectedParams.Revision != params.Revision ||
					expectedParams.Origin != params.Origin ||
					expectedParams.GasPrice != params.GasPrice ||
					!slices.Equal(expectedParams.BlobHashes, params.BlobHashes) ||
					expectedParams.Kind != params.Kind ||
					expectedParams.Static != params.Static ||
					expectedParams.Depth != params.Depth ||
					expectedParams.Gas != params.Gas ||
					expectedParams.Recipient != params.Recipient ||
					expectedParams.Sender != params.Sender ||
					!slices.Equal(expectedParams.Input, params.Input) ||
					expectedParams.Value != params.Value ||
					expectedParams.CodeHash != params.CodeHash ||
					!bytes.Equal(expectedParams.Code, params.Code) {
					t.Errorf("Parameters did not match, expected %v, got %v", params, expectedParams)
				}

				return tosca.Result{Success: success}, nil
			})

			refundShift := uint64(1 << 60)
			stateDb.EXPECT().AddRefund(refundShift)
			if success {
				stateDb.EXPECT().AddRefund(uint64(0))
				stateDb.EXPECT().GetRefund().Return(refundShift)
				stateDb.EXPECT().SubRefund(refundShift)
			}

			contract := geth.NewContract(common.Address(address), common.Address(address), nil, 0, nil)

			_, err := adapter.Run(contract, []byte{}, false)
			if success && err != nil {
				t.Errorf("Unexpected error: %v", err)
			}
			if !success && err == nil {
				t.Errorf("Expected error, got nil")
			}
		})
	}
}

func TestGethAdapter_CorruptValuesReturnErrors(t *testing.T) {
	tests := map[string]struct {
		firstBlock  *big.Int
		baseFee     *big.Int
		chainID     *big.Int
		blobBaseFee *big.Int
		gasPrice    *big.Int
		difficulty  *big.Int
	}{
		"revision": {
			firstBlock: big.NewInt(1000),
		},
		"baseFee": {
			baseFee: big.NewInt(-1),
		},
		"chainID": {
			chainID: big.NewInt(-1),
		},
		"blobBaseFee": {
			blobBaseFee: big.NewInt(-1),
		},
		"gasPrice": {
			gasPrice: big.NewInt(-1),
		},
		"difficulty": {
			difficulty: big.NewInt(-1),
		},
	}

	for name, test := range tests {
		t.Run("unsupported-"+name, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			stateDb := NewMockStateDb(ctrl)
			interpreter := tosca.NewMockInterpreter(ctrl)

			blockNumber := int64(42)
			blockParameters := geth.BlockContext{
				BlockNumber: big.NewInt(blockNumber),
				BaseFee:     test.baseFee,
				BlobBaseFee: test.blobBaseFee,
				Difficulty:  test.difficulty,
			}
			if test.firstBlock == nil {
				test.firstBlock = big.NewInt(1)
			}
			chainConfig := &params.ChainConfig{ChainID: test.chainID, IstanbulBlock: test.firstBlock}
			evm := geth.NewEVM(blockParameters, stateDb, chainConfig, geth.Config{})
			evm.TxContext = geth.TxContext{
				GasPrice: test.gasPrice,
			}

			adapter := &gethInterpreterAdapter{
				evm:         evm,
				interpreter: interpreter,
			}

			stateDb.EXPECT().AddRefund(gomock.Any())

			address := tosca.Address{0x42}
			contract := geth.NewContract(common.Address(address), common.Address(address), nil, 0, nil)

			ret, err := adapter.Run(contract, nil, false)
			require.Error(t, err, "could not convert"+name)
			require.Nil(t, ret, "expected nil return value")
		})
	}
}

func TestGethAdapter_CallForwardsToTheRightKind(t *testing.T) {
	sender := common.Address{0x42}
	recipient := common.Address{0x43}
	codeAddress := common.Address{0x44}
	input := []byte{0x01, 0x02, 0x03}
	gas := uint64(1000)
	value := uint256.NewInt(100)
	salt := uint256.NewInt(200)

	any := gomock.Any()
	tests := map[string]struct {
		kind  tosca.CallKind
		setup func(mock *MockCallContextInterceptor)
	}{
		"call": {
			kind: tosca.Call,
			setup: func(mock *MockCallContextInterceptor) {
				mock.EXPECT().Call(any, sender, recipient, input, gas, value)
			},
		},
		"delegateCall": {
			kind: tosca.DelegateCall,
			setup: func(mock *MockCallContextInterceptor) {
				mock.EXPECT().DelegateCall(any, sender, codeAddress, input, gas)
			},
		},
		"staticCall": {
			kind: tosca.StaticCall,
			setup: func(mock *MockCallContextInterceptor) {
				mock.EXPECT().StaticCall(any, sender, recipient, input, gas)
			},
		},
		"callCode": {
			kind: tosca.CallCode,
			setup: func(mock *MockCallContextInterceptor) {
				mock.EXPECT().CallCode(any, sender, codeAddress, input, gas, value)
			},
		},

		"create": {
			kind: tosca.Create,
			setup: func(mock *MockCallContextInterceptor) {
				mock.EXPECT().Create(any, sender, input, gas, value)
			},
		},
		"create2": {
			kind: tosca.Create2,
			setup: func(mock *MockCallContextInterceptor) {
				mock.EXPECT().Create2(any, sender, input, gas, value, salt)
			},
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			calls := NewMockCallContextInterceptor(ctrl)
			test.setup(calls)

			evm := newEVMWithPassingChainConfig()
			evm.CallInterceptor = calls
			adapter := &runContextAdapter{evm: evm, caller: sender}

			callArguments := tosca.CallParameters{
				Recipient:   tosca.Address(recipient),
				Sender:      tosca.Address(sender),
				Input:       input,
				Gas:         tosca.Gas(gas),
				Value:       tosca.NewValue(value.Uint64()),
				Salt:        tosca.Hash(tosca.NewValue(salt.Uint64())),
				CodeAddress: tosca.Address(codeAddress),
			}
			_, err := adapter.Call(test.kind, callArguments)
			require.NoError(t, err, "call should not return an error")
		})
	}
}

func TestGethAdapter_CallReturnsErrorOnUnsupportedRevision(t *testing.T) {
	chainConfig := &params.ChainConfig{
		ChainID:       big.NewInt(42),
		IstanbulBlock: big.NewInt(42),
	}
	blockContext := geth.BlockContext{
		BlockNumber: big.NewInt(24),
	}
	evm := geth.NewEVM(blockContext, nil, chainConfig, geth.Config{})
	adapter := &runContextAdapter{evm: evm}
	_, err := adapter.Call(tosca.Call, tosca.CallParameters{})
	require.Error(t, err, "unsupported revision")
}

func TestGethAdapter_UnknownErrorsFromCallAreForwarded(t *testing.T) {
	ctrl := gomock.NewController(t)
	calls := NewMockCallContextInterceptor(ctrl)

	any := gomock.Any()
	calls.EXPECT().Call(any, any, any, any, any, any).Return(
		nil, uint64(0), fmt.Errorf("failed"),
	)

	evm := newEVMWithPassingChainConfig()
	evm.CallInterceptor = calls
	adapter := &runContextAdapter{evm: evm}
	_, err := adapter.Call(tosca.Call, tosca.CallParameters{})
	require.Error(t, err, "call should return an error")
}

func TestRunContextAdapter_bigIntToValue(t *testing.T) {
	tests := map[string]struct {
		input         *big.Int
		want          tosca.Value
		expectedError bool
	}{
		"nil": {
			input:         nil,
			want:          tosca.Value{},
			expectedError: false,
		},
		"zero": {
			input:         big.NewInt(0),
			want:          tosca.NewValue(0),
			expectedError: false,
		},
		"positive": {
			input:         big.NewInt(42),
			want:          tosca.NewValue(42),
			expectedError: false,
		},
		"negative": {
			input:         big.NewInt(-42),
			want:          tosca.Value{},
			expectedError: true,
		},
		"overflow": {
			input:         big.NewInt(1).Lsh(big.NewInt(1), 256),
			want:          tosca.Value{},
			expectedError: true,
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			got, err := bigIntToValue(test.input)
			if test.expectedError && err == nil {
				t.Errorf("Expected error, got nil")
			}
			if !test.expectedError && err != nil {
				t.Errorf("Unexpected error: %v", err)
			}
			if got != test.want {
				t.Errorf("Conversion returned wrong value, expected %v, got %v", test.want, got)
			}
		})
	}
}

func TestRunContextAdapter_bigIntToHash(t *testing.T) {
	input := big.NewInt(42)
	want := tosca.Hash(tosca.NewValue(42))
	got, err := bigIntToHash(input)
	if err != nil {
		t.Errorf("Unexpected error: %v", err)
	}
	if got != want {
		t.Errorf("Conversion returned wrong value, expected %v, got %v", want, got)
	}
}

func TestRunContextAdapter_bigIntToWord(t *testing.T) {
	input := big.NewInt(42)
	want := tosca.Word(tosca.NewValue(42))
	got, err := bigIntToWord(input)
	if err != nil {
		t.Errorf("Unexpected error: %v", err)
	}
	if got != want {
		t.Errorf("Conversion returned wrong value, expected %v, got %v", want, got)
	}
}

func TestRunContextAdapter_ConvertRevision(t *testing.T) {
	pragueTime := uint64(1100)
	cancunTime := uint64(1000)
	shanghaiTime := uint64(900)
	parisBlock := big.NewInt(100)
	londonBlock := big.NewInt(90)
	berlinBlock := big.NewInt(80)
	istanbulBlock := big.NewInt(70)

	tests := map[string]struct {
		random *common.Hash
		block  *big.Int
		time   uint64
		want   tosca.Revision
	}{
		"Istanbul": {
			block: istanbulBlock,
			time:  uint64(0),
			want:  tosca.R07_Istanbul,
		},
		"Berlin": {
			block: berlinBlock,
			time:  uint64(0),
			want:  tosca.R09_Berlin,
		},
		"London": {
			block: londonBlock,
			time:  uint64(0),
			want:  tosca.R10_London,
		},
		"Paris": {
			random: &common.Hash{0x42},
			block:  parisBlock,
			time:   uint64(0),
			want:   tosca.R11_Paris,
		},
		"Shanghai": {
			random: &common.Hash{0x42},
			block:  parisBlock,
			time:   shanghaiTime,
			want:   tosca.R12_Shanghai,
		},
		"Cancun": {
			random: &common.Hash{0x42},
			block:  parisBlock,
			time:   cancunTime,
			want:   tosca.R13_Cancun,
		},
		"Prague": {
			random: &common.Hash{0x42},
			block:  parisBlock,
			time:   pragueTime,
			want:   tosca.R14_Prague,
		},
	}

	chainConfig := &params.ChainConfig{
		ChainID:            big.NewInt(42),
		IstanbulBlock:      istanbulBlock,
		LondonBlock:        londonBlock,
		BerlinBlock:        berlinBlock,
		MergeNetsplitBlock: parisBlock,
		ShanghaiTime:       &shanghaiTime,
		CancunTime:         &cancunTime,
		PragueTime:         &pragueTime,
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			evm := geth.NewEVM(geth.BlockContext{Random: test.random}, nil, chainConfig, geth.Config{})
			rules := evm.ChainConfig().Rules(test.block, evm.Context.Random != nil, test.time)
			got, err := convertRevision(rules)
			if err != nil {
				t.Errorf("Unexpected error: %v", err)
			}
			if got != test.want {
				t.Errorf("Conversion returned wrong value, expected %v, got %v", test.want, got)
			}
		})
	}
}

func TestRunContextAdapter_ConvertRevisionReturnsUnsupportedRevisionError(t *testing.T) {
	rules := params.Rules{
		IsHomestead: true,
	}
	_, err := convertRevision(rules)
	targetError := &tosca.ErrUnsupportedRevision{}
	if !errors.As(err, &targetError) {
		t.Errorf("Expected unsupported revision error, got %v", err)
	}
}

func TestRunContextAdapter_gethToVMErrors(t *testing.T) {
	gas := tosca.Gas(42)
	otherError := fmt.Errorf("other error")
	tests := map[string]struct {
		input      error
		wantResult tosca.CallResult
		wantError  error
	}{
		"nil": {
			input: nil,
		},
		"insufficientBalance": {
			input:      geth.ErrInsufficientBalance,
			wantResult: tosca.CallResult{GasLeft: gas},
			wantError:  nil,
		},
		"maxCallDepth": {
			input:      geth.ErrDepth,
			wantResult: tosca.CallResult{GasLeft: gas},
			wantError:  nil,
		},
		"nonceOverflow": {
			input:      geth.ErrNonceUintOverflow,
			wantResult: tosca.CallResult{GasLeft: gas},
			wantError:  nil,
		},
		"OutOfGas": {
			input:      geth.ErrOutOfGas,
			wantResult: tosca.CallResult{},
			wantError:  nil,
		},
		"stackUnderflow": {
			input:      &geth.ErrStackUnderflow{},
			wantResult: tosca.CallResult{},
			wantError:  nil,
		},
		"stackOverflow": {
			input:      &geth.ErrStackOverflow{},
			wantResult: tosca.CallResult{},
			wantError:  nil,
		},
		"invalidOpCode": {
			input:      &geth.ErrInvalidOpCode{},
			wantResult: tosca.CallResult{},
			wantError:  nil,
		},
		"other": {
			input:      otherError,
			wantResult: tosca.CallResult{},
			wantError:  otherError,
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			gotResult, gotErr := gethToVMErrors(test.input, gas)
			if !errors.Is(gotErr, test.wantError) {
				t.Errorf("Unexpected error: expected %v, got %v", test.wantError, gotErr)
			}
			reflect.DeepEqual(gotResult, test.wantResult)
		})
	}
}

func TestRunContextAdapter_AllGethErrorsAreHandled(t *testing.T) {
	// all errors defined in geth/core/vm/gethErrors.go
	gethErrors := []error{
		geth.ErrOutOfGas,
		geth.ErrCodeStoreOutOfGas,
		geth.ErrDepth,
		geth.ErrInsufficientBalance,
		geth.ErrContractAddressCollision,
		geth.ErrExecutionReverted,
		geth.ErrMaxCodeSizeExceeded,
		geth.ErrMaxInitCodeSizeExceeded,
		geth.ErrInvalidJump,
		geth.ErrWriteProtection,
		geth.ErrReturnDataOutOfBounds,
		geth.ErrGasUintOverflow,
		geth.ErrInvalidCode,
		geth.ErrNonceUintOverflow,

		&geth.ErrStackUnderflow{},
		&geth.ErrStackOverflow{},
		&geth.ErrInvalidOpCode{},
	}

	for _, inErr := range gethErrors {
		_, outErr := gethToVMErrors(inErr, tosca.Gas(42))
		if outErr != nil {
			t.Errorf("Unexpected return error %v", outErr)
		}
	}
}

func TestAdapter_ReadOnlyIsSetAndResetCorrectly(t *testing.T) {
	tests := map[string]bool{
		"readOnly":    true,
		"notReadOnly": false,
	}
	recipient := tosca.Address{0x42}
	depth := 42
	gas := uint64(42)
	for name, readOnly := range tests {
		t.Run(name, func(t *testing.T) {
			setGas := encodeReadOnlyInGas(gas, recipient, tosca.R07_Istanbul, readOnly)
			gotReadOnly, unsetGas := decodeReadOnlyFromGas(depth, readOnly, setGas)

			if unsetGas != gas {
				t.Errorf("Gas was not set or unset correctly, expected %v, got %v", gas, unsetGas)
			}
			if gotReadOnly != readOnly {
				t.Errorf("ReadOnly was not set or unset correctly, expected %v, got %v", readOnly, gotReadOnly)
			}
		})
	}
}

func TestGethInterpreterAdapter_RefundShiftIsReverted(t *testing.T) {
	tests := map[string]struct {
		err    error
		refund uint64
	}{
		"noErrorHighRefund": {
			err:    nil,
			refund: 100,
		},
		"noErrorLowRefund": {
			err:    nil,
			refund: 10,
		},
		"errorHighRefund": {
			err:    fmt.Errorf("error"),
			refund: 100,
		},
		"errorLowRefund": {
			err:    fmt.Errorf("error"),
			refund: 10,
		},
	}
	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			ctrl := gomock.NewController(t)
			stateDb := NewMockStateDb(ctrl)

			shift := uint64(42)
			expectedSub := shift
			if test.refund < shift {
				expectedSub = test.refund
			}

			if test.err == nil {
				stateDb.EXPECT().GetRefund().Return(test.refund)
				stateDb.EXPECT().SubRefund(expectedSub)
			}

			undoRefundShift(stateDb, test.err, shift)
		})
	}
}

func TestGethAdapter_IsPrecompiledContractDependsOnRevision(t *testing.T) {
	tests := map[string]struct {
		revision        tosca.Revision
		lastPrecompiled int
	}{
		"istanbul": {
			revision:        tosca.R07_Istanbul,
			lastPrecompiled: 9,
		},
		"berlin": {
			revision:        tosca.R09_Berlin,
			lastPrecompiled: 9,
		},
		"london": {
			revision:        tosca.R10_London,
			lastPrecompiled: 9,
		},
		"paris": {
			revision:        tosca.R11_Paris,
			lastPrecompiled: 9,
		},
		"shanghai": {
			revision:        tosca.R12_Shanghai,
			lastPrecompiled: 9,
		},
		"cancun": {
			revision:        tosca.R13_Cancun,
			lastPrecompiled: 10,
		},
		"prague": {
			revision:        tosca.R14_Prague,
			lastPrecompiled: 17,
		},
	}

	for name, test := range tests {
		t.Run(name, func(t *testing.T) {
			for i := range test.lastPrecompiled + 256 {
				address := uint256.NewInt(uint64(i)).Bytes20()
				got := isPrecompiledContract(address, test.revision)
				if !got && (i > 0 && i <= test.lastPrecompiled) {
					t.Errorf("Expected %v to be precompiled, got %v", address, got)
				}
				if got && (i < 1 || i > test.lastPrecompiled) {
					t.Errorf("Expected %v to not be precompiled, got %v", address, got)
				}
			}
		})
	}

}

func newEVMWithPassingChainConfig() *geth.EVM {
	chainConfig := &params.ChainConfig{
		ChainID:       big.NewInt(42),
		IstanbulBlock: big.NewInt(24),
	}
	blockContext := geth.BlockContext{
		BlockNumber: big.NewInt(24),
	}
	return geth.NewEVM(blockContext, nil, chainConfig, geth.Config{})
}
