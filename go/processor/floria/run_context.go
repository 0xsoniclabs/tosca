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
	if r.depth > MaxRecursiveDepth {
		return errResult, nil
	}
	r.depth++
	defer func() { r.depth-- }()

	if kind == tosca.Call || kind == tosca.CallCode {
		if !canTransferValue(r, parameters.Value, parameters.Sender, &parameters.Recipient) {
			return errResult, nil
		}
	}
	snapshot := r.CreateSnapshot()
	recipient := parameters.Recipient

	if kind == tosca.StaticCall {
		r.static = true
	}

	isStateContract := isStateContract(parameters.CodeAddress)
	isPrecompiled := isPrecompiled(parameters.CodeAddress, r.blockParameters.Revision)

	if kind == tosca.Call &&
		r.blockParameters.Revision >= tosca.R09_Berlin &&
		!isPrecompiled &&
		!isStateContract &&
		!r.AccountExists(parameters.Recipient) &&
		parameters.Value.Cmp(tosca.Value{}) == 0 {
		return tosca.CallResult{Success: true, GasLeft: parameters.Gas}, nil
	}

	if kind == tosca.Call {
		transferValue(r, parameters.Value, parameters.Sender, recipient)
	}

	if kind == tosca.Call && isStateContract {
		result := runStateContract(r, parameters.Sender, parameters.CodeAddress, parameters.Input, parameters.Gas)
		if !result.Success {
			r.RestoreSnapshot(snapshot)
			result.GasLeft = 0
		}
		return result, nil
	}

	if isPrecompiled {
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
	if r.depth > MaxRecursiveDepth {
		return errResult, nil
	}
	r.depth++
	defer func() { r.depth-- }()

	if err := senderCreateSetUp(parameters, r.TransactionContext); err != nil {
		return errResult, nil
	}

	createdAddress, err := createAddress(kind, parameters, r.blockParameters.Revision, r.TransactionContext)
	if err != nil {
		return tosca.CallResult{}, nil
	}

	// The following changes have an impact on the created address.
	// If a check fails the snapshot will be restored and revert all changes on the
	// created address. The nonce increment of the sender is not impacted.
	snapshot := r.CreateSnapshot()
	r.CreateAccount(createdAddress)
	r.SetNonce(createdAddress, 1)

	transferValue(r, parameters.Value, parameters.Sender, createdAddress)

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

	result = checkAndDeployCode(result, createdAddress, snapshot, r.blockParameters.Revision, r)

	return tosca.CallResult{
		Output:         result.Output,
		GasLeft:        result.GasLeft,
		GasRefund:      result.GasRefund,
		Success:        result.Success,
		CreatedAddress: createdAddress,
	}, nil
}

// senderCreateSetUp performs necessary steps before creating a contract.
func senderCreateSetUp(parameters tosca.CallParameters, context tosca.TransactionContext) error {
	if !canTransferValue(context, parameters.Value, parameters.Sender, &parameters.Recipient) {
		return fmt.Errorf("insufficient balance for value transfer")
	}
	if err := incrementNonce(context, parameters.Sender); err != nil {
		return fmt.Errorf("nonce increment failed: %w", err)
	}
	return nil
}

// createAddress generates a new contract address,
// depending on the revision it is added to the access list.
// An error is return in case the address is not empty.
func createAddress(
	kind tosca.CallKind,
	parameters tosca.CallParameters,
	revision tosca.Revision,
	context tosca.TransactionContext,
) (tosca.Address, error) {
	var createdAddress tosca.Address

	switch kind {
	case tosca.Create:
		createdAddress = tosca.Address(crypto.CreateAddress(
			common.Address(parameters.Sender),
			context.GetNonce(parameters.Sender)-1,
		))
	case tosca.Create2:
		initHash := crypto.Keccak256(parameters.Input)
		createdAddress = tosca.Address(crypto.CreateAddress2(
			common.Address(parameters.Sender),
			common.Hash(parameters.Salt),
			initHash[:],
		))
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

// checkAndDeployCode performs the required checks to ensure the code is valid and can be deployed.
// If all checks pass, the code is deployed, in the case of failure the snapshot is restored,
// gas consumed and no result is returned.
func checkAndDeployCode(
	result tosca.Result,
	createdAddress tosca.Address,
	snapshot tosca.Snapshot,
	revision tosca.Revision,
	context tosca.TransactionContext,
) tosca.Result {
	outCode := result.Output
	// check code size
	if len(outCode) > maxCodeSize {
		result.Success = false
	}

	// with eip-3541 code is not allowed to start with 0xEF
	if revision >= tosca.R10_London && len(outCode) > 0 && outCode[0] == 0xEF {
		result.Success = false
	}

	// charge for code deployment
	deploymentCost := tosca.Gas(len(outCode) * createGasCostPerByte)
	if result.GasLeft < deploymentCost {
		result.Success = false
	}
	result.GasLeft -= deploymentCost

	// deploy code or revert snapshot
	if result.Success {
		context.SetCode(createdAddress, tosca.Code(outCode))
	} else {
		context.RestoreSnapshot(snapshot)
		result.GasLeft = 0
		result.Output = nil
	}
	return result
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

func isRevert(result tosca.Result, err error) bool {
	if err == nil && !result.Success && (result.GasLeft > 0 || len(result.Output) > 0) {
		return true
	}
	return false
}

func canTransferValue(
	context tosca.TransactionContext,
	value tosca.Value,
	sender tosca.Address,
	recipient *tosca.Address,
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

func incrementNonce(context tosca.TransactionContext, address tosca.Address) error {
	nonce := context.GetNonce(address)
	if nonce+1 < nonce {
		return fmt.Errorf("nonce overflow")
	}
	context.SetNonce(address, nonce+1)
	return nil
}

// Only to be called after canTransferValue
func transferValue(
	context tosca.TransactionContext,
	value tosca.Value,
	sender tosca.Address,
	recipient tosca.Address,
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
