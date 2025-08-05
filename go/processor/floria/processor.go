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
)

const (
	TxGas                     = 21_000
	TxGasContractCreation     = 53_000
	TxDataNonZeroGasEIP2028   = 16
	TxDataZeroGasEIP2028      = 4
	TxAccessListAddressGas    = 2400
	TxAccessListStorageKeyGas = 1900

	createGasCostPerByte = 200
	maxCodeSize          = 24576
	maxInitCodeSize      = 2 * maxCodeSize

	MaxRecursiveDepth = 1024 // Maximum depth of call/create stack.
)

func init() {
	tosca.RegisterProcessorFactory("floria", newProcessor)
}

func newProcessor(interpreter tosca.Interpreter) tosca.Processor {
	return &processor{
		interpreter: interpreter,
	}
}

type processor struct {
	interpreter tosca.Interpreter
}

func (p *processor) Run(
	blockParameters tosca.BlockParameters,
	transaction tosca.Transaction,
	context tosca.TransactionContext,
) (tosca.Receipt, error) {

	// Check whether the transaction is valid and eligible for processing.
	if err := preCheckTransaction(blockParameters, transaction, context); err != nil {
		// These transactions are not eligible for processing, and can thus not
		// be charged for, and no receipt can be created for them.
		return tosca.Receipt{}, fmt.Errorf("invalid transaction: %w", err)
	}

	// Buy the gas for the transaction.
	gasPrice, gas, err := buyGas(blockParameters, transaction, context)
	if err != nil {
		// If the gas could not be bought, the transaction can not be charged
		// for, can thus not be processed, and no receipt can be created for it.
		return tosca.Receipt{}, fmt.Errorf("failed to buy gas: %w", err)
	}

	// Run the transaction through the interpreter.
	result, err := p.runTransaction(
		blockParameters,
		transaction,
		context,
		gasPrice,
		gas,
	)
	if err != nil {
		// If an error occurred during the transaction execution, it could
		// not be processed correctly. Thus, it can not be charged for,
		// and no receipt can be created for it.
		return tosca.Receipt{}, fmt.Errorf("failed to run transaction: %w", err)
	}

	// Refund the remaining gas to the sender.
	gasUsed := refundGas(blockParameters.Revision, transaction, context, gasPrice, result)

	// Create the receipt for the transaction.
	receipt := tosca.Receipt{
		Success: result.Success,
		GasUsed: gasUsed,
		Output:  result.Output,
		Logs:    context.GetLogs(),
	}
	if result.Success && transaction.Recipient == nil {
		receipt.ContractAddress = &result.CreatedAddress
	}

	return receipt, nil
}

func preCheckTransaction(
	blockParameters tosca.BlockParameters,
	transaction tosca.Transaction,
	context tosca.TransactionContext,
) error {

	if nonceCheck(transaction.Nonce, context.GetNonce(transaction.Sender)) != nil {
		return fmt.Errorf("nonce check failed for transaction from %s: %w", transaction.Sender, nonceCheck(transaction.Nonce, context.GetNonce(transaction.Sender)))
	}

	if eoaCheck(transaction.Sender, context) != nil {
		return fmt.Errorf("transaction sender %s is not an EOA", transaction.Sender)
	}

	if blockParameters.Revision >= tosca.R12_Shanghai && transaction.Recipient == nil &&
		len(transaction.Input) > maxInitCodeSize {
		return fmt.Errorf("input size %d exceeds maximum allowed size %d", len(transaction.Input), maxInitCodeSize)
	}
	return nil
}

func buyGas(
	blockParameters tosca.BlockParameters,
	transaction tosca.Transaction,
	context tosca.TransactionContext,
) (tosca.Value, tosca.Gas, error) {
	// Compute the effective gas price.
	gasPrice, err := calculateGasPrice(blockParameters.BaseFee, transaction.GasFeeCap, transaction.GasTipCap)
	if err != nil {
		return tosca.Value{}, 0, fmt.Errorf("failed to calculate gas price: %w", err)
	}

	// Buy gas for the transaction.
	if err := buyGasInternal(transaction, context, gasPrice); err != nil {
		return tosca.Value{}, 0, fmt.Errorf("failed to buy gas: %w", err)
	}

	// Charge the setup gas for the transaction.
	setupGas := calculateSetupGas(transaction)
	gas := transaction.GasLimit
	if gas < setupGas {
		return tosca.Value{}, 0, fmt.Errorf("insufficient gas for setup: %w", err)
	}
	gas -= setupGas

	return gasPrice, gas, nil
}

func (p *processor) runTransaction(
	blockParameters tosca.BlockParameters,
	transaction tosca.Transaction,
	context tosca.TransactionContext,
	gasPrice tosca.Value,
	gas tosca.Gas,
) (
	tosca.CallResult,
	error,
) {
	transactionParameters := tosca.TransactionParameters{
		Origin:     transaction.Sender,
		GasPrice:   gasPrice,
		BlobHashes: []tosca.Hash{}, // ?
	}

	runContext := runContext{
		floriaContext{context, context.SelfDestruct},
		p.interpreter,
		blockParameters,
		transactionParameters,
		0,
		false,
	}

	if blockParameters.Revision >= tosca.R09_Berlin {
		setUpAccessList(transaction, &runContext, blockParameters.Revision)
	}

	callParameters := callParameters(transaction, gas)
	kind := callKind(transaction)

	if kind == tosca.Call {
		context.SetNonce(transaction.Sender, context.GetNonce(transaction.Sender)+1)
	}

	return runContext.Call(kind, callParameters)
}

func refundGas(
	revision tosca.Revision,
	transaction tosca.Transaction,
	context tosca.TransactionContext,
	gasPrice tosca.Value,
	result tosca.CallResult,
) (used tosca.Gas) {
	gasLeft := calculateGasLeft(transaction, result, revision)

	sender := transaction.Sender
	refundValue := gasPrice.Scale(uint64(gasLeft))
	senderBalance := context.GetBalance(sender)
	senderBalance = tosca.Add(senderBalance, refundValue)
	context.SetBalance(sender, senderBalance)

	return transaction.GasLimit - gasLeft
}

// ----

func calculateGasPrice(baseFee, gasFeeCap, gasTipCap tosca.Value) (tosca.Value, error) {
	if gasFeeCap.Cmp(baseFee) < 0 {
		return tosca.Value{}, fmt.Errorf("gasFeeCap %v is lower than baseFee %v", gasFeeCap, baseFee)
	}
	return tosca.Add(baseFee, tosca.Min(gasTipCap, tosca.Sub(gasFeeCap, baseFee))), nil
}

// floriaContext is a wrapper around the tosca.TransactionContext
// that adds the balance transfer to the selfdestruct function
type floriaContext struct {
	tosca.TransactionContext
	// the original selfdestruct function is saved here, as it still needs to be called
	selfdestruct func(addr, beneficiary tosca.Address) bool
}

func (c floriaContext) SelfDestruct(addr tosca.Address, beneficiary tosca.Address) bool {
	c.SetBalance(beneficiary, tosca.Add(c.GetBalance(beneficiary), c.GetBalance(addr)))
	return c.selfdestruct(addr, beneficiary)
}

func nonceCheck(transactionNonce uint64, stateNonce uint64) error {
	if transactionNonce != stateNonce {
		return fmt.Errorf("nonce mismatch: %v != %v", transactionNonce, stateNonce)
	}
	if stateNonce+1 < stateNonce {
		return fmt.Errorf("nonce overflow")
	}
	return nil
}

// Only accept transactions from externally owned accounts (EOAs) and not from contracts
func eoaCheck(sender tosca.Address, context tosca.TransactionContext) error {
	codehash := context.GetCodeHash(sender)
	if codehash != (tosca.Hash{}) && codehash != emptyCodeHash {
		return fmt.Errorf("sender is not an EOA")
	}
	return nil
}

func setUpAccessList(transaction tosca.Transaction, context tosca.TransactionContext, revision tosca.Revision) {
	if transaction.AccessList == nil {
		return
	}

	context.AccessAccount(transaction.Sender)
	if transaction.Recipient != nil {
		context.AccessAccount(*transaction.Recipient)
	}

	precompiles := getPrecompiledAddresses(revision)
	for _, address := range precompiles {
		context.AccessAccount(address)
	}

	for _, accessTuple := range transaction.AccessList {
		context.AccessAccount(accessTuple.Address)
		for _, key := range accessTuple.Keys {
			context.AccessStorage(accessTuple.Address, key)
		}
	}
}

func callKind(transaction tosca.Transaction) tosca.CallKind {
	if transaction.Recipient == nil {
		return tosca.Create
	}
	return tosca.Call
}

func callParameters(transaction tosca.Transaction, gas tosca.Gas) tosca.CallParameters {
	callParameters := tosca.CallParameters{
		Sender: transaction.Sender,
		Input:  transaction.Input,
		Value:  transaction.Value,
		Gas:    gas,
	}
	if transaction.Recipient != nil {
		callParameters.Recipient = *transaction.Recipient
	}
	return callParameters
}

func calculateGasLeft(transaction tosca.Transaction, result tosca.CallResult, revision tosca.Revision) tosca.Gas {
	gasLeft := result.GasLeft

	// 10% of remaining gas is charged for non-internal transactions
	if transaction.Sender != (tosca.Address{}) {
		gasLeft -= gasLeft / 10
	}

	if result.Success {
		gasUsed := transaction.GasLimit - gasLeft
		refund := result.GasRefund

		maxRefund := tosca.Gas(0)
		if revision < tosca.R10_London {
			// Before EIP-3529: refunds were capped to gasUsed / 2
			maxRefund = gasUsed / 2
		} else {
			// After EIP-3529: refunds are capped to gasUsed / 5
			maxRefund = gasUsed / 5
		}

		if refund > maxRefund {
			refund = maxRefund
		}
		gasLeft += refund
	}

	return gasLeft
}

func calculateSetupGas(transaction tosca.Transaction) tosca.Gas {
	var gas tosca.Gas
	if transaction.Recipient == nil {
		gas = TxGasContractCreation
	} else {
		gas = TxGas
	}

	if len(transaction.Input) > 0 {
		nonZeroBytes := tosca.Gas(0)
		for _, inputByte := range transaction.Input {
			if inputByte != 0 {
				nonZeroBytes++
			}
		}
		zeroBytes := tosca.Gas(len(transaction.Input)) - nonZeroBytes

		// No overflow check for the gas computation is required although it is performed in the
		// opera version. The overflow check would be triggered in a worst case with an input
		// greater than 2^64 / 16 - 53000 = ~10^18, which is not possible with real world hardware
		gas += zeroBytes * TxDataZeroGasEIP2028
		gas += nonZeroBytes * TxDataNonZeroGasEIP2028
	}

	if transaction.AccessList != nil {
		gas += tosca.Gas(len(transaction.AccessList)) * TxAccessListAddressGas

		// charge for each storage key
		for _, accessTuple := range transaction.AccessList {
			gas += tosca.Gas(len(accessTuple.Keys)) * TxAccessListStorageKeyGas
		}
	}

	return tosca.Gas(gas)
}

func buyGasInternal(transaction tosca.Transaction, context tosca.TransactionContext, gasPrice tosca.Value) error {
	gas := gasPrice.Scale(uint64(transaction.GasLimit))

	// Buy gas
	senderBalance := context.GetBalance(transaction.Sender)
	if senderBalance.Cmp(gas) < 0 {
		return fmt.Errorf("insufficient balance: %v < %v", senderBalance, gas)
	}

	senderBalance = tosca.Sub(senderBalance, gas)
	context.SetBalance(transaction.Sender, senderBalance)

	return nil
}
