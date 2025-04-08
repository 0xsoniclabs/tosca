// Copyright (c) 2024 Fantom Foundation
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at fantom.foundation/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package floria

import (
	"fmt"
	"math/big"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/ethereum/go-ethereum/crypto/kzg4844"
	"github.com/holiman/uint256"
)

const (
	TxGas                     = 21_000
	TxGasContractCreation     = 53_000
	TxDataNonZeroGasEIP2028   = 16
	TxDataZeroGasEIP2028      = 4
	TxAccessListAddressGas    = 2400
	TxAccessListStorageKeyGas = 1900
	InitCodeWordGas           = 2 // Once per word of the init code when creating a contract.

	createGasCostPerByte = 200
	maxCodeSize          = 24576
	maxInitCodeSize      = 2 * maxCodeSize

	BlobTxBlobGasPerBlob = 1 << 17

	MaxRecursiveDepth = 1024 // Maximum depth of call/create stack.
)

func init() {
	tosca.RegisterProcessorFactory("floria", newProcessor)
}

func newProcessor(interpreter tosca.Interpreter) tosca.Processor {
	return &Processor{
		Interpreter:        interpreter,
		EthereumCompatible: false,
	}
}

type Processor struct {
	Interpreter        tosca.Interpreter
	EthereumCompatible bool
}

func (p *Processor) Run(
	blockParameters tosca.BlockParameters,
	transaction tosca.Transaction,
	context tosca.TransactionContext,
) (tosca.Receipt, error) {
	snapshot := context.CreateSnapshot()
	errorReceipt := tosca.Receipt{
		Success: false,
		GasUsed: transaction.GasLimit,
	}
	gasPrice, err := calculateGasPrice(blockParameters.BaseFee, transaction.GasFeeCap, transaction.GasTipCap)
	if err != nil {
		return errorReceipt, err
	}

	if nonceCheck(transaction.Nonce, context.GetNonce(transaction.Sender)) != nil {
		if p.EthereumCompatible {
			return tosca.Receipt{}, fmt.Errorf("nonce mismatch")
		}
		return tosca.Receipt{}, nil
	}

	if eoaCheck(transaction.Sender, context) != nil {
		if p.EthereumCompatible {
			return tosca.Receipt{}, fmt.Errorf("sender is not an eoa")
		}
		return tosca.Receipt{}, nil
	}

	if err = blobCheck(transaction, blockParameters, context); err != nil {
		if p.EthereumCompatible {
			return errorReceipt, err
		}
		return errorReceipt, nil
	}

	if err := buyGas(transaction, context, gasPrice, blockParameters.BlobBaseFee, p.EthereumCompatible); err != nil {
		context.RestoreSnapshot(snapshot)
		if p.EthereumCompatible {
			return tosca.Receipt{}, fmt.Errorf("insufficient balance for gas")
		}
		return tosca.Receipt{}, nil
	}

	gas := transaction.GasLimit
	setupGas := calculateSetupGas(transaction, blockParameters.Revision)
	if gas < setupGas {
		context.RestoreSnapshot(snapshot)
		if p.EthereumCompatible {
			return tosca.Receipt{}, fmt.Errorf("insufficient gas for set up")
		}
		return errorReceipt, nil
	}
	gas -= setupGas

	if blockParameters.Revision >= tosca.R12_Shanghai && transaction.Recipient == nil &&
		len(transaction.Input) > maxInitCodeSize {
		context.RestoreSnapshot(snapshot)
		if p.EthereumCompatible {
			return tosca.Receipt{}, fmt.Errorf("max init code size exceeded")
		}
		return tosca.Receipt{}, nil
	}

	transactionParameters := tosca.TransactionParameters{
		Origin:     transaction.Sender,
		GasPrice:   gasPrice,
		BlobHashes: transaction.BlobHashes,
	}

	runContext := runContext{
		floriaContext{context, blockParameters.Revision, context.SelfDestruct},
		p.Interpreter,
		blockParameters,
		transactionParameters,
		0,
		false,
	}

	if blockParameters.Revision >= tosca.R09_Berlin {
		setUpAccessList(transaction, &runContext, blockParameters)
	}

	callParameters := callParameters(transaction, gas)
	kind := callKind(transaction)

	if kind == tosca.Call {
		context.SetNonce(transaction.Sender, context.GetNonce(transaction.Sender)+1)
	}

	result, err := runContext.Call(kind, callParameters)
	if err != nil {
		return errorReceipt, err
	}

	var createdAddress *tosca.Address
	if kind == tosca.Create {
		createdAddress = &result.CreatedAddress
	}

	gasLeft := calculateGasLeft(transaction, result, blockParameters.Revision, p.EthereumCompatible)
	refundGas(context, transaction.Sender, gasPrice, gasLeft)

	if p.EthereumCompatible {
		paymentToCoinbase(transaction, gasPrice, transaction.GasLimit-gasLeft, blockParameters, context)
	}

	logs := context.GetLogs()

	return tosca.Receipt{
		Success:         result.Success,
		GasUsed:         transaction.GasLimit - gasLeft,
		ContractAddress: createdAddress,
		Output:          result.Output,
		Logs:            logs,
	}, nil
}

func calculateGasPrice(baseFee, gasFeeCap, gasTipCap tosca.Value) (tosca.Value, error) {
	if gasFeeCap.Cmp(baseFee) < 0 {
		return tosca.Value{}, fmt.Errorf("gasFeeCap %v is lower than baseFee %v", gasFeeCap, baseFee)
	}
	if gasFeeCap.Cmp(gasTipCap) < 0 {
		return tosca.Value{}, fmt.Errorf("gasFeeCap %v is lower than gasTipCap %v", gasFeeCap, gasTipCap)
	}
	return tosca.Add(baseFee, tosca.Min(gasTipCap, tosca.Sub(gasFeeCap, baseFee))), nil
}

// floriaContext is a wrapper around the tosca.TransactionContext
// that adds the balance transfer to the selfdestruct function
type floriaContext struct {
	tosca.TransactionContext
	revision tosca.Revision
	// the original selfdestruct function is saved here, as it still needs to be called
	selfdestruct func(addr, beneficiary tosca.Address) bool
}

func (c floriaContext) SelfDestruct(addr tosca.Address, beneficiary tosca.Address) bool {
	balance := c.GetBalance(addr)
	if c.revision >= tosca.R13_Cancun {
		c.SetBalance(addr, tosca.Value{})
	}
	c.SetBalance(beneficiary, tosca.Add(c.GetBalance(beneficiary), balance))
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

func setUpAccessList(transaction tosca.Transaction, context tosca.TransactionContext, blockParameters tosca.BlockParameters) {
	if transaction.AccessList == nil {
		return
	}

	context.AccessAccount(transaction.Sender)
	if transaction.Recipient != nil {
		context.AccessAccount(*transaction.Recipient)
	}

	precompiles := getPrecompiledAddresses(blockParameters.Revision)
	for _, address := range precompiles {
		context.AccessAccount(address)
	}

	for _, accessTuple := range transaction.AccessList {
		context.AccessAccount(accessTuple.Address)
		for _, key := range accessTuple.Keys {
			context.AccessStorage(accessTuple.Address, key)
		}
	}

	if blockParameters.Revision >= tosca.R12_Shanghai {
		context.AccessAccount(blockParameters.Coinbase)
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
		callParameters.CodeAddress = *transaction.Recipient
	}
	return callParameters
}

func calculateGasLeft(transaction tosca.Transaction, result tosca.CallResult, revision tosca.Revision, ethCompatible bool) tosca.Gas {
	gasLeft := result.GasLeft

	// 10% of remaining gas is charged for non-internal transactions
	if !ethCompatible && transaction.Sender != (tosca.Address{}) {
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

func refundGas(context tosca.TransactionContext, sender tosca.Address, gasPrice tosca.Value, gasLeft tosca.Gas) {
	refundValue := gasPrice.Scale(uint64(gasLeft))
	senderBalance := context.GetBalance(sender)
	senderBalance = tosca.Add(senderBalance, refundValue)
	context.SetBalance(sender, senderBalance)
}

func calculateSetupGas(transaction tosca.Transaction, revision tosca.Revision) tosca.Gas {
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

		if transaction.Recipient == nil && revision >= tosca.R12_Shanghai {
			lenWords := tosca.SizeInWords(uint64(len(transaction.Input)))
			gas += tosca.Gas(lenWords * InitCodeWordGas)
		}
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

func buyGas(transaction tosca.Transaction, context tosca.TransactionContext, gasPrice tosca.Value, blobBaseFee tosca.Value, ethCompatible bool) error {
	gas := gasPrice.Scale(uint64(transaction.GasLimit))

	if ethCompatible {
		if err := ethereumBalanceCheck(gasPrice, transaction, context); err != nil {
			return err
		}
	}

	if len(transaction.BlobHashes) > 0 {
		blobFee := blobBaseFee.Scale(uint64(len(transaction.BlobHashes) * BlobTxBlobGasPerBlob))
		gas = tosca.Add(gas, blobFee)
	}

	// Buy gas
	senderBalance := context.GetBalance(transaction.Sender)
	if senderBalance.Cmp(gas) < 0 {
		return fmt.Errorf("insufficient balance: %v < %v", senderBalance, gas)
	}

	senderBalance = tosca.Sub(senderBalance, gas)
	context.SetBalance(transaction.Sender, senderBalance)

	return nil
}

func ethereumBalanceCheck(gasPrice tosca.Value, transaction tosca.Transaction, context tosca.TransactionContext) error {
	capGas := gasPrice.ToBig().Mul(gasPrice.ToBig(), big.NewInt(int64(transaction.GasLimit)))
	if transaction.GasFeeCap != (tosca.Value{}) {
		capGas = transaction.GasFeeCap.ToBig().Mul(transaction.GasFeeCap.ToBig(), big.NewInt(int64(transaction.GasLimit)))
	}

	capGas = capGas.Add(capGas, transaction.Value.ToBig())

	if len(transaction.BlobHashes) > 0 {
		blobFee := transaction.BlobGasFeeCap.Scale(uint64(len(transaction.BlobHashes) * BlobTxBlobGasPerBlob))
		capGas = capGas.Add(capGas, blobFee.ToBig())
	}

	capGasU256, overflow := uint256.FromBig(capGas)
	if overflow {
		return fmt.Errorf("capGas overflow")
	}
	capGasValue := tosca.ValueFromUint256(capGasU256)

	if have := context.GetBalance(transaction.Sender); have.Cmp(capGasValue) < 0 {
		return fmt.Errorf("insufficient balance: %v < %v", have, capGasValue)
	}

	return nil
}

func blobCheck(transaction tosca.Transaction, blockParameters tosca.BlockParameters, context tosca.TransactionContext) error {
	if transaction.BlobHashes != nil {
		if transaction.Recipient == nil {
			return fmt.Errorf("blob transactions need to have a existing recipient")
		}
		if len(transaction.BlobHashes) == 0 {
			return fmt.Errorf("missing blob hashes")
		}
		for _, hash := range transaction.BlobHashes {
			if !kzg4844.IsValidVersionedHash(hash[:]) {
				return fmt.Errorf("blob with invalid hash version")
			}
		}

	}

	if blockParameters.Revision >= tosca.R13_Cancun && len(transaction.BlobHashes) > 0 {
		if transaction.BlobGasFeeCap == (tosca.Value{}) {
			return nil // skip checks if no blob gas fee cap is set
		}
		if transaction.BlobGasFeeCap.Cmp(blockParameters.BlobBaseFee) < 0 {
			return fmt.Errorf("blobGasFeeCap %v is lower than blobBaseFee %v", transaction.BlobGasFeeCap, blockParameters.BlobBaseFee)
		}
	}
	return nil
}

func paymentToCoinbase(transaction tosca.Transaction, gasPrice tosca.Value, gasUsed tosca.Gas, blockParameters tosca.BlockParameters, context tosca.TransactionContext) {
	if transaction.GasFeeCap == (tosca.Value{}) && transaction.GasTipCap == (tosca.Value{}) {
		return
	}
	effectiveTip := gasPrice
	if blockParameters.Revision >= tosca.R10_London {
		effectiveTip = tosca.Sub(transaction.GasFeeCap, blockParameters.BaseFee)
		if effectiveTip.Cmp(transaction.GasTipCap) > 0 {
			effectiveTip = transaction.GasTipCap
		}
	}
	fee := effectiveTip.Scale(uint64(gasUsed))
	context.SetBalance(blockParameters.Coinbase, tosca.Add(context.GetBalance(blockParameters.Coinbase), fee))
}
