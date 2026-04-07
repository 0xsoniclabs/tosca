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
	"github.com/0xsoniclabs/tosca/go/tosca"
	"github.com/ethereum/go-ethereum/common"
	state "github.com/ethereum/go-ethereum/core/state"
	"github.com/ethereum/go-ethereum/core/stateless"
	"github.com/ethereum/go-ethereum/core/tracing"
	"github.com/ethereum/go-ethereum/core/types"
	vm "github.com/ethereum/go-ethereum/core/vm"
	"github.com/ethereum/go-ethereum/params"
	"github.com/holiman/uint256"
)

type ToscaStateDB interface {
	vm.StateDB
	GetRefund() uint64
	SetRefund(refund uint64)
}

// ProcessorStateDB is a wrapper around the tosca.TransactionContext to implement the vm.ProcessorStateDB interface.
type ProcessorStateDB struct {
	InterpreterStateDB
	context         tosca.ProcessorContext
	createdContract *common.Address
	snapshots       []snapshot
}

// snapshot combines the snapshot ID of the underlying transaction context with
// a backup of the StateDB's refund state.
type snapshot struct {
	snapshot tosca.Snapshot
	refund   uint64
}

func NewStateDB(ctx tosca.ProcessorContext) *ProcessorStateDB {
	return &ProcessorStateDB{
		InterpreterStateDB: *NewInterpreterStateDB(ctx),
		context:            ctx,
	}
}

func (s *ProcessorStateDB) GetCreatedContract() *common.Address {
	return s.createdContract
}

func (s *ProcessorStateDB) GetLogs() []types.Log {
	logs := make([]types.Log, 0)
	for _, log := range s.context.GetLogs() {
		topics := make([]common.Hash, len(log.Topics))
		for i, topic := range log.Topics {
			topics[i] = common.Hash(topic)
		}
		logs = append(logs, types.Log{
			Address: common.Address(log.Address),
			Topics:  topics,
			Data:    log.Data,
		})
	}
	return logs
}

// vm.StateDB interface implementation

func (s *ProcessorStateDB) CreateAccount(common.Address) {
	// not implemented
}

func (s *ProcessorStateDB) CreateContract(address common.Address) {
	s.createdContract = &address
	s.context.CreateContract(tosca.Address(address))
}

func (s *ProcessorStateDB) SetNonce(address common.Address, nonce uint64, _ tracing.NonceChangeReason) {
	s.context.SetNonce(tosca.Address(address), nonce)
}

func (s *ProcessorStateDB) SetCode(address common.Address, code []byte, _ tracing.CodeChangeReason) []byte {
	prevCode := s.context.GetCode(tosca.Address(address))
	s.context.SetCode(tosca.Address(address), code)
	return prevCode
}

func (s *ProcessorStateDB) GetRefund() uint64 {
	return s.refund
}

func (s *ProcessorStateDB) GetStorageRoot(address common.Address) common.Hash {
	if s.context.HasEmptyStorage(tosca.Address(address)) {
		return common.Hash{}
	}
	return common.Hash{0x42} // non empty root hash
}

func (s *ProcessorStateDB) Exist(address common.Address) bool {
	return s.context.AccountExists(tosca.Address(address))
}

func (s *ProcessorStateDB) Prepare(rules params.Rules, sender, coinbase common.Address, dest *common.Address, precompiles []common.Address, txAccesses types.AccessList) {
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

func (s *ProcessorStateDB) RevertToSnapshot(snapshot int) {
	if snapshot < 0 || snapshot >= len(s.snapshots) {
		return
	}
	s.context.RestoreSnapshot(s.snapshots[snapshot].snapshot)
	s.refund = s.snapshots[snapshot].refund
}

func (s *ProcessorStateDB) Snapshot() int {
	id := s.context.CreateSnapshot()
	s.snapshots = append(s.snapshots, snapshot{
		snapshot: tosca.Snapshot(id),
		refund:   s.refund,
	})
	return len(s.snapshots) - 1
}

type InterpreterStateDB struct {
	context     tosca.InterpreterContext
	refund      uint64
	beneficiary common.Address
}

func NewInterpreterStateDB(ctx tosca.InterpreterContext) *InterpreterStateDB {
	return &InterpreterStateDB{context: ctx}
}

func (s *InterpreterStateDB) SetRefund(refund uint64) {
	s.refund = refund
}

func (s *InterpreterStateDB) GetRefund() uint64 {
	return s.refund
}

// vm.StateDB interface implementation

func (s *InterpreterStateDB) GetBalance(address common.Address) *uint256.Int {
	return s.context.GetBalance(tosca.Address(address)).ToUint256()
}

func (s *InterpreterStateDB) GetNonce(address common.Address) uint64 {
	return s.context.GetNonce(tosca.Address(address))
}

func (s *InterpreterStateDB) GetCodeHash(address common.Address) common.Hash {
	return common.Hash(s.context.GetCodeHash(tosca.Address(address)))
}

func (s *InterpreterStateDB) GetCode(address common.Address) []byte {
	return s.context.GetCode(tosca.Address(address))
}

func (s *InterpreterStateDB) GetCodeSize(address common.Address) int {
	return s.context.GetCodeSize(tosca.Address(address))
}

func (s *InterpreterStateDB) AddRefund(refund uint64) {
	s.refund += refund
}

func (s *InterpreterStateDB) SubRefund(refund uint64) {
	s.refund -= refund
}

func (s *InterpreterStateDB) GetState(address common.Address, key common.Hash) common.Hash {
	return common.Hash(s.context.GetStorage(tosca.Address(address), tosca.Key(key)))
}

func (s *InterpreterStateDB) SetState(address common.Address, key common.Hash, value common.Hash) common.Hash {
	state := s.context.GetStorage(tosca.Address(address), tosca.Key(key))
	s.context.SetStorage(tosca.Address(address), tosca.Key(key), tosca.Word(value))
	return common.Hash(state)
}

func (s *InterpreterStateDB) GetTransientState(address common.Address, key common.Hash) common.Hash {
	return common.Hash(s.context.GetTransientStorage(tosca.Address(address), tosca.Key(key)))
}

func (s *InterpreterStateDB) SetTransientState(address common.Address, key, value common.Hash) {
	s.context.SetTransientStorage(tosca.Address(address), tosca.Key(key), tosca.Word(value))
}

func (s *InterpreterStateDB) SelfDestruct(address common.Address) {
	s.context.SelfDestruct(tosca.Address(address), tosca.Address(s.beneficiary))
}

// HasSelfDestructed should only be used by geth_adapter
func (s *InterpreterStateDB) HasSelfDestructed(address common.Address) bool {
	return s.context.HasSelfDestructed(tosca.Address(address))
}

func (s *InterpreterStateDB) Empty(address common.Address) bool {
	return s.context.GetBalance(tosca.Address(address)) == tosca.NewValue(0) &&
		s.context.GetNonce(tosca.Address(address)) == 0 &&
		s.context.GetCodeSize(tosca.Address(address)) == 0
}

// AddressInAccessList should only be used by geth_adapter
func (s *InterpreterStateDB) AddressInAccessList(address common.Address) bool {
	return s.context.IsAddressInAccessList(tosca.Address(address))
}

// SlotInAccessList should only be used by geth_adapter
func (s *InterpreterStateDB) SlotInAccessList(address common.Address, slot common.Hash) (addressOk bool, slotOk bool) {
	return s.context.IsSlotInAccessList(tosca.Address(address), tosca.Key(slot))
}

func (s *InterpreterStateDB) AddAddressToAccessList(address common.Address) {
	s.context.AccessAccount(tosca.Address(address))
}

func (s *InterpreterStateDB) AddSlotToAccessList(address common.Address, slot common.Hash) {
	s.context.AccessStorage(tosca.Address(address), tosca.Key(slot))
}

func (s *InterpreterStateDB) GetStateAndCommittedState(address common.Address, key common.Hash) (common.Hash, common.Hash) {
	state := common.Hash(s.context.GetStorage(tosca.Address(address), tosca.Key(key)))
	committedState := common.Hash(s.context.GetCommittedStorage(tosca.Address(address), tosca.Key(key)))
	return state, committedState
}

func (s *InterpreterStateDB) AddLog(log *types.Log) {
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

func (s *InterpreterStateDB) IsNewContract(address common.Address) bool {
	return s.context.IsNewContract(tosca.Address(address))
}

func (s *InterpreterStateDB) SubBalance(address common.Address, value *uint256.Int, tracing tracing.BalanceChangeReason) uint256.Int {
	balance := s.context.GetBalance(tosca.Address(address))
	s.context.SetBalance(tosca.Address(address), tosca.Sub(balance, tosca.ValueFromUint256(value)))
	return *balance.ToUint256()
}

func (s *InterpreterStateDB) AddBalance(address common.Address, value *uint256.Int, tracing tracing.BalanceChangeReason) uint256.Int {
	balance := s.context.GetBalance(tosca.Address(address))
	s.context.SetBalance(tosca.Address(address), tosca.Add(balance, tosca.ValueFromUint256(value)))

	// In the case of a seldestruct the balance is transferred to the beneficiary,
	// we save this address for the context-selfdestruct call.
	// this only works if the balance transfer is performed before the selfdestruct call,
	// as it is the performed in geth and the geth adapter.
	s.beneficiary = address
	return *balance.ToUint256()
}

func (s *InterpreterStateDB) SetNonce(address common.Address, nonce uint64, _ tracing.NonceChangeReason) {
	panic("not implemented")
}

func (s *InterpreterStateDB) SetCode(address common.Address, code []byte, _ tracing.CodeChangeReason) []byte {
	panic("not implemented")
}

func (s *InterpreterStateDB) Exist(address common.Address) bool {
	panic("not implemented")
}

func (s *InterpreterStateDB) CreateAccount(common.Address) {
	panic("not implemented")
}

func (s *InterpreterStateDB) GetStorageRoot(address common.Address) common.Hash {
	panic("not implemented")
}

func (s *InterpreterStateDB) CreateContract(address common.Address) {
	panic("not implemented")
}

func (s *InterpreterStateDB) Prepare(rules params.Rules, sender, coinbase common.Address, dest *common.Address, precompiles []common.Address, txAccesses types.AccessList) {
	panic("not implemented")
}

func (s *InterpreterStateDB) RevertToSnapshot(snapshot int) {
	panic("not implemented")
}

func (s *InterpreterStateDB) Snapshot() int {
	panic("not implemented")
}

func (s *InterpreterStateDB) AddPreimage(common.Hash, []byte) {
	panic("not implemented")
}

func (s *InterpreterStateDB) Witness() *stateless.Witness {
	return nil
}

func (s *InterpreterStateDB) AccessEvents() *state.AccessEvents {
	panic("not implemented")
}

func (s *InterpreterStateDB) Finalise(bool) {
	panic("not implemented")
}
