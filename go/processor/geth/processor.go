// Copyright (c) 2024 Fantom Foundation
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at fantom.foundation/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package geth_processor

import (
	"errors"
	"fmt"
	"math/big"

	"github.com/0xsoniclabs/tosca/go/geth_adapter"
	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/holiman/uint256"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core"
	"github.com/ethereum/go-ethereum/core/tracing"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/params"
)

func init() {
	tosca.RegisterProcessorFactory("geth-sonic", sonicProcessor)
}

func sonicProcessor(interpreter tosca.Interpreter) tosca.Processor {
	return &Processor{
		interpreter:        interpreter,
		ethereumCompatible: false,
	}
}

type Processor struct {
	interpreter        tosca.Interpreter
	ethereumCompatible bool
}

func (p *Processor) Run(
	blockParameters tosca.BlockParameters,
	transaction tosca.Transaction,
	context tosca.TransactionContext,
) (tosca.Receipt, error) {
	gasPrice, err := calculateGasPrice(blockParameters.BaseFee, transaction.GasFeeCap, transaction.GasTipCap)
	if err != nil {
		return tosca.Receipt{}, err
	}

	blockContext := newBlockContext(blockParameters, context)

	var blobHashes []common.Hash
	if transaction.BlobHashes != nil {
		blobHashes = make([]common.Hash, len(transaction.BlobHashes))
		for i, hash := range transaction.BlobHashes {
			blobHashes[i] = common.Hash(hash)
		}
	}

	txContext := vm.TxContext{
		Origin:     common.Address(transaction.Sender),
		GasPrice:   gasPrice.ToBig(),
		BlobHashes: blobHashes,
		BlobFeeCap: transaction.BlobGasFeeCap.ToBig(),
	}
	stateDB := geth_adapter.NewStateDB(context)
	chainConfig := blockParametersToChainConfig(blockParameters)
	config := newEVMConfig(p.interpreter, p.ethereumCompatible)
	evm := vm.NewEVM(blockContext, txContext, stateDB, chainConfig, config)

	msg := transactionToMessage(transaction, gasPrice, blobHashes)
	gasPool := new(core.GasPool).AddGas(uint64(transaction.GasLimit))
	result, err := core.ApplyMessage(evm, msg, gasPool)
	if err != nil {
		if !p.ethereumCompatible && errors.Is(err, core.ErrInsufficientFunds) {
			return tosca.Receipt{}, err
		}
		return tosca.Receipt{GasUsed: transaction.GasLimit}, err
	}

	createdAddress := (*tosca.Address)(stateDB.GetCreatedContract())
	if transaction.Recipient != nil || result.Failed() {
		createdAddress = nil
	}

	logs := make([]tosca.Log, 0)
	for _, log := range stateDB.GetLogs() {
		topics := make([]tosca.Hash, len(log.Topics))
		for i, topic := range log.Topics {
			topics[i] = tosca.Hash(topic)
		}
		logs = append(logs, tosca.Log{
			Address: tosca.Address(log.Address),
			Topics:  topics,
			Data:    log.Data,
		})
	}

	return tosca.Receipt{
		Success:         !result.Failed(),
		Output:          result.ReturnData,
		ContractAddress: createdAddress,
		GasUsed:         tosca.Gas(result.UsedGas),
		Logs:            logs,
	}, nil
}

func calculateGasPrice(baseFee, gasFeeCap, gasTipCap tosca.Value) (tosca.Value, error) {
	if gasFeeCap.Cmp(baseFee) < 0 {
		return tosca.Value{}, fmt.Errorf("gasFeeCap %v is lower than baseFee %v", gasFeeCap, baseFee)
	}
	return tosca.Add(baseFee, tosca.Min(gasTipCap, tosca.Sub(gasFeeCap, baseFee))), nil
}

func newBlockContext(blockParameters tosca.BlockParameters, context tosca.TransactionContext) vm.BlockContext {
	canTransfer := func(stateDB vm.StateDB, address common.Address, value *uint256.Int) bool {
		return stateDB.GetBalance(address).Cmp(value) >= 0
	}

	transfer := func(stateDB vm.StateDB, sender common.Address, recipient common.Address, value *uint256.Int) {
		stateDB.SubBalance(sender, value, tracing.BalanceChangeTransfer)
		stateDB.AddBalance(recipient, value, tracing.BalanceChangeTransfer)
	}

	hashFunc := func(num uint64) common.Hash {
		return common.Hash(context.GetBlockHash(int64(num)))
	}

	sonicDifficulty := big.NewInt(1)

	return vm.BlockContext{
		CanTransfer: canTransfer,
		Transfer:    transfer,
		GetHash:     hashFunc,
		Coinbase:    common.Address(blockParameters.Coinbase),
		GasLimit:    uint64(blockParameters.GasLimit),
		BlockNumber: new(big.Int).SetInt64(blockParameters.BlockNumber),
		Time:        uint64(blockParameters.Timestamp),
		Difficulty:  sonicDifficulty,
		BaseFee:     blockParameters.BaseFee.ToBig(),
		BlobBaseFee: blockParameters.BlobBaseFee.ToBig(),
		Random:      (*common.Hash)(&blockParameters.PrevRandao),
	}
}

func blockParametersToChainConfig(blockParams tosca.BlockParameters) *params.ChainConfig {
	chainConfig := *params.AllEthashProtocolChanges
	chainConfig.ChainID = new(big.Int).SetBytes(blockParams.ChainID[:])
	chainConfig.ByzantiumBlock = big.NewInt(0)
	chainConfig.IstanbulBlock = big.NewInt(0)
	chainConfig.BerlinBlock = big.NewInt(0)
	chainConfig.LondonBlock = big.NewInt(0)
	chainConfig.MergeNetsplitBlock = big.NewInt(0)
	zeroTime := uint64(0)
	chainConfig.ShanghaiTime = &zeroTime
	chainConfig.CancunTime = &zeroTime

	greaterBlockTime := uint64(blockParams.Timestamp + 1)
	greaterBlockNumber := big.NewInt(blockParams.BlockNumber + 1)

	if blockParams.Revision < tosca.R13_Cancun {
		chainConfig.CancunTime = &greaterBlockTime
	}
	if blockParams.Revision < tosca.R12_Shanghai {
		chainConfig.ShanghaiTime = &greaterBlockTime
	}
	if blockParams.Revision < tosca.R11_Paris {
		chainConfig.MergeNetsplitBlock = greaterBlockNumber
	}
	if blockParams.Revision < tosca.R10_London {
		chainConfig.LondonBlock = greaterBlockNumber
	}
	if blockParams.Revision < tosca.R09_Berlin {
		chainConfig.BerlinBlock = greaterBlockNumber
	}
	return &chainConfig
}

func newEVMConfig(interpreter tosca.Interpreter, ethereumCompatible bool) vm.Config {
	config := vm.Config{
		StatePrecompiles: map[common.Address]vm.PrecompiledStateContract{
			stateContractAddress: PreCompiledContract{},
		},
		Interpreter: geth_adapter.NewGethInterpreterFactory(interpreter),
	}
	if !ethereumCompatible {
		config.ChargeExcessGas = true
		config.IgnoreGasFeeCap = true
		config.InsufficientBalanceIsNotAnError = true
		config.SkipTipPaymentToCoinbase = true
	}
	return config
}

func transactionToMessage(transaction tosca.Transaction, gasPrice tosca.Value, blobHashes []common.Hash) *core.Message {
	accessList := types.AccessList{}
	for _, tuple := range transaction.AccessList {
		storageKeys := make([]common.Hash, len(tuple.Keys))
		for i, key := range tuple.Keys {
			storageKeys[i] = common.Hash(key)
		}
		accessList = append(accessList, types.AccessTuple{
			Address:     common.Address(tuple.Address),
			StorageKeys: storageKeys,
		})
	}

	return &core.Message{
		From:              common.Address(transaction.Sender),
		To:                (*common.Address)(transaction.Recipient),
		Nonce:             transaction.Nonce,
		Value:             transaction.Value.ToBig(),
		GasLimit:          uint64(transaction.GasLimit),
		GasPrice:          gasPrice.ToBig(),
		GasFeeCap:         transaction.GasFeeCap.ToBig(),
		GasTipCap:         transaction.GasTipCap.ToBig(),
		Data:              transaction.Input,
		AccessList:        accessList,
		BlobGasFeeCap:     transaction.BlobGasFeeCap.ToBig(),
		BlobHashes:        blobHashes,
		SkipAccountChecks: false,
	}
}
