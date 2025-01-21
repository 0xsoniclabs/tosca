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
	"math/big"

	"github.com/0xsoniclabs/tosca/go/geth_adapter"
	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/holiman/uint256"

	"github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core"
	"github.com/ethereum/go-ethereum/core/stateless"
	"github.com/ethereum/go-ethereum/core/tracing"
	"github.com/ethereum/go-ethereum/core/types"
	"github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/params"
	"github.com/ethereum/go-ethereum/trie/utils"
)

func init() {
	tosca.RegisterProcessorFactory("geth-sonic", fantomProcessor)
}

func fantomProcessor(interpreter tosca.Interpreter) tosca.Processor {
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
	blockContext := newBlockContext(blockParameters, context)
	txContext := vm.TxContext{
		Origin:     common.Address(transaction.Sender),
		GasPrice:   transaction.GasPrice.ToBig(),
		BlobFeeCap: blockParameters.BlobBaseFee.ToBig(),
	}
	stateDB := &stateDB{context: context}
	chainConfig := blockParametersToChainConfig(blockParameters)
	config := newEVMConfig(p.interpreter, p.ethereumCompatible)
	evm := vm.NewEVM(blockContext, txContext, stateDB, chainConfig, config)

	msg := transactionToMessage(transaction, blockParameters.BaseFee)
	gasPool := new(core.GasPool).AddGas(uint64(transaction.GasLimit))
	result, err := core.ApplyMessage(evm, msg, gasPool)
	if err != nil {
		if !p.ethereumCompatible && errors.Is(err, core.ErrInsufficientFunds) {
			return tosca.Receipt{}, err
		}
		return tosca.Receipt{GasUsed: transaction.GasLimit}, err
	}

	createdAddress := (*tosca.Address)(&stateDB.createdContract)
	if transaction.Recipient != nil || result.Failed() {
		createdAddress = nil
	}

	return tosca.Receipt{
		Success:         !result.Failed(),
		Output:          result.ReturnData,
		ContractAddress: createdAddress,
		GasUsed:         tosca.Gas(result.UsedGas),
		Logs:            stateDB.context.GetLogs(),
	}, nil
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

func transactionToMessage(transaction tosca.Transaction, baseFee tosca.Value) *core.Message {
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
		From:     common.Address(transaction.Sender),
		To:       (*common.Address)(transaction.Recipient),
		Nonce:    transaction.Nonce,
		Value:    transaction.Value.ToBig(),
		GasLimit: uint64(transaction.GasLimit),
		GasPrice: transaction.GasPrice.ToBig(),
		// gas price computation and enforcement of cap is currently performed outside of processor
		// TODO: extend the tosca.Transaction to include GasFeeCap and GasTipCap
		GasFeeCap:  big.NewInt(0).Add(baseFee.ToBig(), big.NewInt(1)),
		GasTipCap:  big.NewInt(0),
		Data:       transaction.Input,
		AccessList: accessList,
		// TODO: add support for blobs in the tosca.Transaction
		BlobGasFeeCap:     big.NewInt(0),
		BlobHashes:        nil,
		SkipAccountChecks: false,
	}
}

// stateDB is a wrapper around the tosca.TransactionContext to implement the vm.StateDB interface.
// TODO: merge with tosca/go/interpreter/geth/geth.go stateDbAdapter
type stateDB struct {
	context         tosca.TransactionContext
	refund          uint64
	createdContract common.Address
	refundBackups   map[tosca.Snapshot]uint64
	beneficiary     common.Address
}

// vm.StateDB interface implementation

func (s *stateDB) CreateAccount(common.Address) {
	// not implemented
}

func (s *stateDB) CreateContract(address common.Address) {
	s.createdContract = address
}

func (s *stateDB) SubBalance(address common.Address, value *uint256.Int, tracing tracing.BalanceChangeReason) {
	toscaAddress := tosca.Address(address)
	balance := s.context.GetBalance(toscaAddress)
	s.context.SetBalance(toscaAddress, tosca.Sub(balance, tosca.ValueFromUint256(value)))
}

func (s *stateDB) AddBalance(address common.Address, value *uint256.Int, tracing tracing.BalanceChangeReason) {
	toscaAddress := tosca.Address(address)
	balance := s.context.GetBalance(toscaAddress)
	s.context.SetBalance(toscaAddress, tosca.Add(balance, tosca.ValueFromUint256(value)))

	// In the case of a seldestruct the balance is transferred to the beneficiary,
	// we save this address for the context-selfdestruct call.
	// this only works if the balance transfer is performed before the selfdestruct call,
	// as it is the performed in geth and the geth adapter.
	s.beneficiary = address
}

func (s *stateDB) GetBalance(address common.Address) *uint256.Int {
	return s.context.GetBalance(tosca.Address(address)).ToUint256()
}

func (s *stateDB) GetNonce(address common.Address) uint64 {
	return s.context.GetNonce(tosca.Address(address))
}

func (s *stateDB) SetNonce(address common.Address, nonce uint64) {
	s.context.SetNonce(tosca.Address(address), nonce)
}

func (s *stateDB) GetCodeHash(address common.Address) common.Hash {
	return common.Hash(s.context.GetCodeHash(tosca.Address(address)))
}

func (s *stateDB) GetCode(address common.Address) []byte {
	return s.context.GetCode(tosca.Address(address))
}

func (s *stateDB) SetCode(address common.Address, code []byte) {
	s.context.SetCode(tosca.Address(address), code)
}

func (s *stateDB) GetCodeSize(address common.Address) int {
	return len(s.GetCode(address))
}

func (s *stateDB) AddRefund(refund uint64) {
	s.refund += refund
}

func (s *stateDB) SubRefund(refund uint64) {
	s.refund -= refund
}

func (s *stateDB) GetRefund() uint64 {
	return s.refund
}

func (s *stateDB) GetCommittedState(address common.Address, key common.Hash) common.Hash {
	//lint:ignore SA1019 deprecated functions to be migrated
	return common.Hash(s.context.GetCommittedStorage(tosca.Address(address), tosca.Key(key)))
}

func (s *stateDB) GetState(address common.Address, key common.Hash) common.Hash {
	return common.Hash(s.context.GetStorage(tosca.Address(address), tosca.Key(key)))
}

func (s *stateDB) SetState(address common.Address, key common.Hash, value common.Hash) {
	s.context.SetStorage(tosca.Address(address), tosca.Key(key), tosca.Word(value))
}

func (s *stateDB) GetStorageRoot(address common.Address) common.Hash {
	return common.Hash{} // not implemented
}

func (s *stateDB) GetTransientState(address common.Address, key common.Hash) common.Hash {
	return common.Hash(s.context.GetTransientStorage(tosca.Address(address), tosca.Key(key)))
}

func (s *stateDB) SetTransientState(address common.Address, key, value common.Hash) {
	s.context.SetTransientStorage(tosca.Address(address), tosca.Key(key), tosca.Word(value))
}

func (s *stateDB) SelfDestruct(address common.Address) {
	s.context.SelfDestruct(tosca.Address(address), tosca.Address(s.beneficiary))
}

func (s *stateDB) HasSelfDestructed(address common.Address) bool {
	//lint:ignore SA1019 deprecated functions to be migrated
	return s.context.HasSelfDestructed(tosca.Address(address))
}

func (s *stateDB) Selfdestruct6780(address common.Address) {
	s.context.SelfDestruct(tosca.Address(address), tosca.Address(s.beneficiary))
}

func (s *stateDB) Exist(address common.Address) bool {
	return s.context.AccountExists(tosca.Address(address))
}

func (s *stateDB) Empty(address common.Address) bool {
	return s.context.GetBalance(tosca.Address(address)) == tosca.NewValue(0) &&
		s.context.GetNonce(tosca.Address(address)) == 0 &&
		len(s.context.GetCode(tosca.Address(address))) == 0
}

func (s *stateDB) AddressInAccessList(address common.Address) bool {
	// using the non-deprecated function has a side effect of adding the address to the access list
	return bool(s.context.AccessAccount(tosca.Address(address)))
}

func (s *stateDB) SlotInAccessList(address common.Address, slot common.Hash) (addressOk bool, slotOk bool) {
	// using the non-deprecated functions has a side effect of adding the address and slot to the access list
	addressOk = bool(s.context.AccessAccount(tosca.Address(address)))
	slotOk = bool(s.context.AccessStorage(tosca.Address(address), tosca.Key(slot)))
	return addressOk, slotOk
}

func (s *stateDB) AddAddressToAccessList(address common.Address) {
	s.context.AccessAccount(tosca.Address(address))
}

func (s *stateDB) AddSlotToAccessList(address common.Address, slot common.Hash) {
	s.context.AccessStorage(tosca.Address(address), tosca.Key(slot))
}

func (s *stateDB) PointCache() *utils.PointCache {
	panic("not implemented")
}

func (s *stateDB) Prepare(rules params.Rules, sender, coinbase common.Address, dest *common.Address, precompiles []common.Address, txAccesses types.AccessList) {
	if rules.IsBerlin {
		s.context.AccessAccount(tosca.Address(sender))
		if dest != nil {
			s.context.AccessAccount(tosca.Address(*dest))
		}
		for _, addr := range precompiles {
			s.context.AccessAccount(tosca.Address(addr))
		}
		for _, el := range txAccesses {
			s.context.AccessAccount(tosca.Address(el.Address))
			for _, key := range el.StorageKeys {
				s.context.AccessStorage(tosca.Address(el.Address), tosca.Key(key))
			}
		}

		if rules.IsShanghai {
			s.context.AccessAccount(tosca.Address(coinbase))
		}
	}
}

func (s *stateDB) RevertToSnapshot(snapshot int) {
	s.context.RestoreSnapshot(tosca.Snapshot(snapshot))
	s.refund = s.refundBackups[tosca.Snapshot(snapshot)]
}

func (s *stateDB) Snapshot() int {
	id := s.context.CreateSnapshot()
	if s.refundBackups == nil {
		s.refundBackups = make(map[tosca.Snapshot]uint64)
	}
	s.refundBackups[id] = s.refund
	return int(id)
}

func (s *stateDB) AddLog(log *types.Log) {
	topics := make([]tosca.Hash, len(log.Topics))
	for i, topic := range log.Topics {
		topics[i] = tosca.Hash(topic)
	}
	toscaLog := tosca.Log{
		Address: tosca.Address(log.Address),
		Topics:  topics,
		Data:    log.Data,
	}
	s.context.EmitLog(tosca.Log(toscaLog))
}

func (s *stateDB) AddPreimage(common.Hash, []byte) {
	panic("not implemented")
}

func (s *stateDB) Witness() *stateless.Witness {
	return nil
}
