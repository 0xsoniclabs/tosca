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

func (r runContext) executeCall(kind tosca.CallKind, parameters tosca.CallParameters) (callResult tosca.CallResult, err error) {
	errResult := tosca.CallResult{
		Success: false,
		GasLeft: parameters.Gas,
	}

	// The runContext is passed to the interpreter by value,
	// therefore no decrement of depth is required.
	if r.incrementDepth() != nil {
		return errResult, nil
	}

	// Just like the depth, the static flag does not need to be reset.
	r.static = r.static || kind == tosca.StaticCall

	snapshot := r.CreateSnapshot()
	defer func() {
		// For all returns with an error or an unsuccessful result,
		// the snapshot will be restored.
		if err != nil || !callResult.Success {
			r.RestoreSnapshot(snapshot)
		}
	}()

	if kind == tosca.Call || kind == tosca.CallCode {
		if !canTransferValue(r, parameters.Value, parameters.Sender, &parameters.Recipient) {
			return errResult, nil
		}
		transferValue(r, parameters.Value, parameters.Sender, parameters.Recipient)
	}

	if kind == tosca.Call && isStateContract(parameters.CodeAddress) {
		result :=
			runStateContract(r, parameters.Sender, parameters.CodeAddress, parameters.Input, parameters.Gas)
		return result, nil
	}

	if isPrecompiled(parameters.CodeAddress, r.blockParameters.Revision) {
		result, err :=
			runPrecompiledContract(r.blockParameters.Revision, parameters.Input, parameters.CodeAddress, parameters.Gas)
		if err != nil {
			result.Success = false
		}
		return result, nil
	}

	result, err := r.runInterpreter(kind, parameters)
	if err != nil {
		return tosca.CallResult{}, err
	}

	return tosca.CallResult{
		Output:    result.Output,
		GasLeft:   result.GasLeft,
		GasRefund: result.GasRefund,
		Success:   result.Success,
	}, nil
}

func (r runContext) executeCreate(kind tosca.CallKind, parameters tosca.CallParameters) (callResult tosca.CallResult, err error) {
	errResult := tosca.CallResult{
		Success: false,
		GasLeft: parameters.Gas,
	}
	if r.incrementDepth() != nil {
		return errResult, nil
	}

	if err := senderCreateSetUp(parameters, r.TransactionContext); err != nil {
		// the set up only fails if the create can not be executed in the current state,
		// a unsuccessful receipt is returned but no gas is consumed.
		return errResult, nil
	}

	createdAddress, err := createAddress(kind, parameters, r.blockParameters.Revision, r.TransactionContext)
	if err != nil {
		// the address has been generated, therefore the gas is consumed in case of an error.
		return tosca.CallResult{}, nil
	}

	// The following changes have an impact on the created address.
	// If a check fails the snapshot will be restored and revert all changes on the
	// created address. The nonce increment of the sender is not impacted.
	snapshot := r.CreateSnapshot()
	defer func() {
		// For all returns with an error or an unsuccessful result,
		// the snapshot will be restored.
		if err != nil || !callResult.Success {
			r.RestoreSnapshot(snapshot)
		}
	}()

	r.CreateAccount(createdAddress)
	r.SetNonce(createdAddress, 1)

	transferValue(r, parameters.Value, parameters.Sender, createdAddress)

	parameters.Recipient = createdAddress
	result, err := r.runInterpreter(kind, parameters)
	if err != nil {
		return tosca.CallResult{}, err
	}
	if !result.Success {
		return tosca.CallResult{
			Output:         result.Output,
			GasLeft:        result.GasLeft,
			GasRefund:      result.GasRefund,
			CreatedAddress: createdAddress,
		}, nil
	}

	result = checkAndDeployCode(result, createdAddress, r.blockParameters.Revision, r)

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
	default:
		return tosca.Address{}, fmt.Errorf("invalid call kind for create: %d", kind)
	}

	if revision >= tosca.R09_Berlin {
		context.AccessAccount(createdAddress)
	}

	if !isEmpty(context, createdAddress) {
		return tosca.Address{}, fmt.Errorf("created address is not empty")
	}

	return createdAddress, nil
}

// isEmpty checks whether an account has no nonce update, no code and empty storage.
func isEmpty(context tosca.TransactionContext, address tosca.Address) bool {
	return context.GetNonce(address) == 0 && context.HasEmptyStorage(address) &&
		(context.GetCodeHash(address) == (tosca.Hash{}) ||
			context.GetCodeHash(address) == emptyCodeHash)
}

// checkAndDeployCode performs the required checks to ensure the code is valid and can be deployed.
// If all checks pass, the code is deployed, in the case of failure the snapshot is restored and
// the gas consumed.
func checkAndDeployCode(
	result tosca.Result,
	createdAddress tosca.Address,
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

// incrementDepth increases the depth of the run context.
// In case the maximum call depth is exceeded, an error is returned.
func (r *runContext) incrementDepth() error {
	if r.depth > MaxRecursiveDepth {
		return fmt.Errorf("max recursive depth reached")
	}
	r.depth++
	return nil
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
