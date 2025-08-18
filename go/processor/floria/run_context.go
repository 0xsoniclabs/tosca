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

	"github.com/0xsoniclabs/tosca/go/tosca"

	// geth dependencies
	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/crypto"
)

var emptyCodeHash = tosca.Hash(crypto.Keccak256(nil))

type runContext struct {
	tosca.TransactionContext
	interpreter           tosca.Interpreter
	blockParameters       tosca.BlockParameters
	transactionParameters tosca.TransactionParameters
	depth                 int
	static                bool
}

func (r runContext) Call(kind tosca.CallKind, parameters tosca.CallParameters) (tosca.CallResult, error) {
	if kind == tosca.Create || kind == tosca.Create2 {
		return r.executeCreate(kind, parameters)
	}
	return r.executeCall(kind, parameters)
}

func (r runContext) executeCall(kind tosca.CallKind, parameters tosca.CallParameters) (tosca.CallResult, error) {
	errResult := tosca.CallResult{
		Success: false,
		GasLeft: parameters.Gas,
	}
	decrementDepthCounter, err := r.incrementDepthCounter()
	if err != nil {
		return errResult, nil
	}
	defer decrementDepthCounter()

	// only set and reset static if it was not set before.
	if !r.static && kind == tosca.StaticCall {
		r.static = true
		defer func() { r.static = false }()
	}

	snapshot := r.CreateSnapshot()

	if kind == tosca.Call || kind == tosca.CallCode {
		if !canTransferValue(parameters.Value, parameters.Sender, &parameters.Recipient, r) {
			return errResult, nil
		}
		if kind == tosca.Call {
			transferValue(parameters.Value, parameters.Sender, parameters.Recipient, r)
		}
	}

	if kind == tosca.Call && isStateContract(parameters.CodeAddress) {
		result := runStateContract(r, parameters.Sender, parameters.CodeAddress, parameters.Input, parameters.Gas)
		if !result.Success {
			r.RestoreSnapshot(snapshot)
			result.GasLeft = 0
		}
		return result, nil
	}

	if isPrecompiled(parameters.CodeAddress, r.blockParameters.Revision) {
		result, err := runPrecompiledContract(r.blockParameters.Revision, parameters.Input, parameters.CodeAddress, parameters.Gas)
		if err != nil {
			r.RestoreSnapshot(snapshot)
		}
		return result, nil
	}

	callResult, err := r.runInterpreter(kind, parameters)
	if err != nil || !callResult.Success {
		r.RestoreSnapshot(snapshot)

		if !isRevert(callResult, err) {
			// if the unsuccessful call was due to a revert, the gas is not consumed
			callResult.GasLeft = 0
		}
	}

	return tosca.CallResult{
		Output:    callResult.Output,
		GasLeft:   callResult.GasLeft,
		GasRefund: callResult.GasRefund,
		Success:   callResult.Success,
	}, err
}

func (r runContext) executeCreate(kind tosca.CallKind, parameters tosca.CallParameters) (tosca.CallResult, error) {
	errResult := tosca.CallResult{
		Success: false,
		GasLeft: parameters.Gas,
	}
	decrementDepthCounter, err := r.incrementDepthCounter()
	if err != nil {
		return errResult, nil
	}
	defer decrementDepthCounter()

	if err := createPreChecks(parameters, r.TransactionContext); err != nil {
		return errResult, nil
	}

	createdAddress, err := createAddress(kind, parameters, r.blockParameters.Revision, r.TransactionContext)
	if err != nil {
		return tosca.CallResult{}, nil
	}

	snapshot := r.CreateSnapshot()
	r.CreateAccount(createdAddress)
	r.SetNonce(createdAddress, 1)

	transferValue(parameters.Value, parameters.Sender, createdAddress, r)

	parameters.Recipient = createdAddress
	result, err := r.runInterpreter(kind, parameters)
	if err != nil || !result.Success {
		r.RestoreSnapshot(snapshot)

		if !isRevert(result, err) {
			// if the unsuccessful create was due to a revert, the result is still returned
			return tosca.CallResult{}, err
		}
		return tosca.CallResult{Output: result.Output, GasLeft: result.GasLeft, CreatedAddress: createdAddress}, nil
	}

	result = finalizeCreate(result, createdAddress, snapshot, r.blockParameters.Revision, r.TransactionContext)

	return tosca.CallResult{
		Output:         result.Output,
		GasLeft:        result.GasLeft,
		GasRefund:      result.GasRefund,
		Success:        result.Success,
		CreatedAddress: createdAddress,
	}, nil
}

func (r *runContext) incrementDepthCounter() (func(), error) {
	if r.depth > MaxRecursiveDepth {
		return func() {}, fmt.Errorf("maximum recursive depth exceeded")
	}
	r.depth++
	return func() { r.depth-- }, nil
}

func (r runContext) runInterpreter(kind tosca.CallKind, parameters tosca.CallParameters) (tosca.Result, error) {
	var code tosca.Code
	var codeHash tosca.Hash
	switch kind {
	case tosca.Call, tosca.StaticCall:
		code = r.GetCode(parameters.Recipient)
		codeHash = r.GetCodeHash(parameters.Recipient)
	case tosca.CallCode, tosca.DelegateCall:
		code = r.GetCode(parameters.CodeAddress)
		codeHash = r.GetCodeHash(parameters.CodeAddress)
	case tosca.Create, tosca.Create2:
		code = tosca.Code(parameters.Input)
		codeHash = tosca.Hash(crypto.Keccak256(code))
		parameters.Input = nil
	}

	interpreterParameters := tosca.Parameters{
		BlockParameters:       r.blockParameters,
		TransactionParameters: r.transactionParameters,
		Context:               r,
		Static:                r.static,
		Depth:                 r.depth - 1, // depth has already been incremented
		Gas:                   parameters.Gas,
		Recipient:             parameters.Recipient,
		Sender:                parameters.Sender,
		Input:                 parameters.Input,
		Value:                 parameters.Value,
		CodeHash:              &codeHash,
		Code:                  code,
	}

	return r.interpreter.Run(interpreterParameters)
}

func createPreChecks(parameters tosca.CallParameters, context tosca.TransactionContext) error {
	if !canTransferValue(parameters.Value, parameters.Sender, &parameters.Recipient, context) {
		return fmt.Errorf("insufficient balance for value transfer")
	}
	if err := incrementNonce(parameters.Sender, context); err != nil {
		return err
	}
	return nil
}

func createAddress(kind tosca.CallKind, parameters tosca.CallParameters, revision tosca.Revision, context tosca.TransactionContext) (tosca.Address, error) {
	var createdAddress tosca.Address
	if kind == tosca.Create {
		createdAddress = tosca.Address(crypto.CreateAddress(common.Address(parameters.Sender), context.GetNonce(parameters.Sender)-1))
	} else {
		initHash := crypto.Keccak256(parameters.Input)
		createdAddress = tosca.Address(crypto.CreateAddress2(common.Address(parameters.Sender), common.Hash(parameters.Salt), initHash[:]))
	}

	if revision >= tosca.R09_Berlin {
		context.AccessAccount(createdAddress)
	}

	if context.GetNonce(createdAddress) != 0 ||
		!context.HasEmptyStorage(createdAddress) ||
		(context.GetCodeHash(createdAddress) != (tosca.Hash{}) &&
			context.GetCodeHash(createdAddress) != emptyCodeHash) {
		return tosca.Address{}, fmt.Errorf("created address is not empty")
	}

	return createdAddress, nil
}

func finalizeCreate(result tosca.Result, createdAddress tosca.Address, snapshot tosca.Snapshot, revision tosca.Revision, context tosca.TransactionContext) tosca.Result {
	outCode := result.Output
	if len(outCode) > maxCodeSize {
		result.Success = false
	}
	if revision >= tosca.R10_London && len(outCode) > 0 && outCode[0] == 0xEF {
		result.Success = false
	}
	createGas := tosca.Gas(len(outCode) * createGasCostPerByte)
	if result.GasLeft < createGas {
		result.Success = false
	}
	result.GasLeft -= createGas

	if result.Success {
		context.SetCode(createdAddress, tosca.Code(outCode))
	} else {
		context.RestoreSnapshot(snapshot)
		result.GasLeft = 0
		result.Output = nil
	}
	return result
}

func isRevert(result tosca.Result, err error) bool {
	if err == nil && !result.Success && (result.GasLeft > 0 || len(result.Output) > 0) {
		return true
	}
	return false
}

func canTransferValue(
	value tosca.Value,
	sender tosca.Address,
	recipient *tosca.Address,
	context tosca.TransactionContext,
) bool {
	if value == (tosca.Value{}) {
		return true
	}

	senderBalance := context.GetBalance(sender)
	if senderBalance.Cmp(value) < 0 {
		return false
	}

	if recipient == nil || sender == *recipient {
		return true
	}

	receiverBalance := context.GetBalance(*recipient)
	updatedBalance := tosca.Add(receiverBalance, value)
	if updatedBalance.Cmp(receiverBalance) < 0 || updatedBalance.Cmp(value) < 0 {
		return false
	}

	return true
}

func incrementNonce(address tosca.Address, context tosca.TransactionContext) error {
	nonce := context.GetNonce(address)
	if nonce+1 < nonce {
		return fmt.Errorf("nonce overflow")
	}
	context.SetNonce(address, nonce+1)
	return nil
}

// Only to be called after canTransferValue
func transferValue(
	value tosca.Value,
	sender tosca.Address,
	recipient tosca.Address,
	context tosca.TransactionContext,
) {
	if value == (tosca.Value{}) {
		return
	}
	if sender == recipient {
		return
	}

	senderBalance := context.GetBalance(sender)
	receiverBalance := context.GetBalance(recipient)
	updatedBalance := tosca.Add(receiverBalance, value)

	senderBalance = tosca.Sub(senderBalance, value)
	context.SetBalance(sender, senderBalance)
	context.SetBalance(recipient, updatedBalance)
}
