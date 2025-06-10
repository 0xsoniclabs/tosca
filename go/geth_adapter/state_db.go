// Copyright (c) 2024 Fantom Foundation
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at fantom.foundation/bsl11.
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
	"github.com/ethereum/go-ethereum/params"
	"github.com/ethereum/go-ethereum/trie/utils"
	"github.com/holiman/uint256"
)

// StateDB is a wrapper around the tosca.TransactionContext to implement the vm.StateDB interface.
type StateDB struct {
	context         tosca.TransactionContext
	createdContract *common.Address
	refund          uint64
	refundBackups   map[tosca.Snapshot]uint64
	beneficiary     common.Address
}

func NewStateDB(ctx tosca.TransactionContext) *StateDB {
	return &StateDB{context: ctx}
}

func (s *StateDB) GetCreatedContract() *common.Address {
	return s.createdContract
}

func (s *StateDB) SetRefund(refund uint64) {
	s.refund = refund
}

func (s *StateDB) GetLogs() []types.Log {
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

func (s *StateDB) CreateAccount(common.Address) {
	// not implemented
}

func (s *StateDB) CreateContract(address common.Address) {
	s.createdContract = &address
	s.context.CreateAccount(tosca.Address(address))
}

func (s *StateDB) SubBalance(address common.Address, value *uint256.Int, tracing tracing.BalanceChangeReason) uint256.Int {
	toscaAddress := tosca.Address(address)
	balance := s.context.GetBalance(toscaAddress)
	s.context.SetBalance(toscaAddress, tosca.Sub(balance, tosca.ValueFromUint256(value)))
	return *balance.ToUint256()
}

func (s *StateDB) AddBalance(address common.Address, value *uint256.Int, tracing tracing.BalanceChangeReason) uint256.Int {
	toscaAddress := tosca.Address(address)
	balance := s.context.GetBalance(toscaAddress)
	s.context.SetBalance(toscaAddress, tosca.Add(balance, tosca.ValueFromUint256(value)))

	// In the case of a seldestruct the balance is transferred to the beneficiary,
	// we save this address for the context-selfdestruct call.
	// this only works if the balance transfer is performed before the selfdestruct call,
	// as it is the performed in geth and the geth adapter.
	s.beneficiary = address
	return *balance.ToUint256()
}

func (s *StateDB) GetBalance(address common.Address) *uint256.Int {
	return s.context.GetBalance(tosca.Address(address)).ToUint256()
}

func (s *StateDB) GetNonce(address common.Address) uint64 {
	return s.context.GetNonce(tosca.Address(address))
}

func (s *StateDB) SetNonce(address common.Address, nonce uint64, _ tracing.NonceChangeReason) {
	s.context.SetNonce(tosca.Address(address), nonce)
}

func (s *StateDB) GetCodeHash(address common.Address) common.Hash {
	return common.Hash(s.context.GetCodeHash(tosca.Address(address)))
}

func (s *StateDB) GetCode(address common.Address) []byte {
	return s.context.GetCode(tosca.Address(address))
}

func (s *StateDB) SetCode(address common.Address, code []byte) []byte {
	oldCode := s.context.GetCode(tosca.Address(address))
	s.context.SetCode(tosca.Address(address), code)
	return oldCode
}

func (s *StateDB) GetCodeSize(address common.Address) int {
	return s.context.GetCodeSize(tosca.Address(address))
}

func (s *StateDB) AddRefund(refund uint64) {
	s.refund += refund
}

func (s *StateDB) SubRefund(refund uint64) {
	s.refund -= refund
}

func (s *StateDB) GetRefund() uint64 {
	return s.refund
}

// GetCommittedState should only be used by geth_adapter
func (s *StateDB) GetCommittedState(address common.Address, key common.Hash) common.Hash {
	return common.Hash(s.context.GetCommittedStorage(tosca.Address(address), tosca.Key(key)))
}

func (s *StateDB) GetState(address common.Address, key common.Hash) common.Hash {
	return common.Hash(s.context.GetStorage(tosca.Address(address), tosca.Key(key)))
}

func (s *StateDB) SetState(address common.Address, key common.Hash, value common.Hash) common.Hash {
	state := s.context.GetStorage(tosca.Address(address), tosca.Key(key))
	s.context.SetStorage(tosca.Address(address), tosca.Key(key), tosca.Word(value))
	return common.Hash(state)
}

func (s *StateDB) GetStorageRoot(address common.Address) common.Hash {
	if s.context.HasEmptyStorage(tosca.Address(address)) {
		return common.Hash{}
	}
	return common.Hash{0x42} // non empty root hash
}

func (s *StateDB) GetTransientState(address common.Address, key common.Hash) common.Hash {
	return common.Hash(s.context.GetTransientStorage(tosca.Address(address), tosca.Key(key)))
}

func (s *StateDB) SetTransientState(address common.Address, key, value common.Hash) {
	s.context.SetTransientStorage(tosca.Address(address), tosca.Key(key), tosca.Word(value))
}

func (s *StateDB) SelfDestruct(address common.Address) uint256.Int {
	balance := s.context.GetBalance(tosca.Address(address))
	s.context.SelfDestruct(tosca.Address(address), tosca.Address(s.beneficiary))
	return *balance.ToUint256()
}

// HasSelfDestructed should only be used by geth_adapter
func (s *StateDB) HasSelfDestructed(address common.Address) bool {
	return s.context.HasSelfDestructed(tosca.Address(address))
}

func (s *StateDB) SelfDestruct6780(address common.Address) (uint256.Int, bool) {
	balance := s.context.GetBalance(tosca.Address(address))
	hasSelfDestructed := s.context.SelfDestruct(tosca.Address(address), tosca.Address(s.beneficiary))
	return *balance.ToUint256(), hasSelfDestructed
}

func (s *StateDB) Exist(address common.Address) bool {
	return s.context.AccountExists(tosca.Address(address))
}

func (s *StateDB) Empty(address common.Address) bool {
	return s.context.GetBalance(tosca.Address(address)) == tosca.NewValue(0) &&
		s.context.GetNonce(tosca.Address(address)) == 0 &&
		s.context.GetCodeSize(tosca.Address(address)) == 0
}

// AddressInAccessList should only be used by geth_adapter
func (s *StateDB) AddressInAccessList(address common.Address) bool {
	return s.context.IsAddressInAccessList(tosca.Address(address))
}

// SlotInAccessList should only be used by geth_adapter
func (s *StateDB) SlotInAccessList(address common.Address, slot common.Hash) (addressOk bool, slotOk bool) {
	return s.context.IsSlotInAccessList(tosca.Address(address), tosca.Key(slot))
}

func (s *StateDB) AddAddressToAccessList(address common.Address) {
	s.context.AccessAccount(tosca.Address(address))
}

func (s *StateDB) AddSlotToAccessList(address common.Address, slot common.Hash) {
	s.context.AccessStorage(tosca.Address(address), tosca.Key(slot))
}

func (s *StateDB) PointCache() *utils.PointCache {
	panic("not implemented")
}

func (s *StateDB) Prepare(rules params.Rules, sender, coinbase common.Address, dest *common.Address, precompiles []common.Address, txAccesses types.AccessList) {
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

func (s *StateDB) RevertToSnapshot(snapshot int) {
	s.context.RestoreSnapshot(tosca.Snapshot(snapshot))
	s.refund = s.refundBackups[tosca.Snapshot(snapshot)]
}

func (s *StateDB) Snapshot() int {
	id := s.context.CreateSnapshot()
	if s.refundBackups == nil {
		s.refundBackups = make(map[tosca.Snapshot]uint64)
	}
	s.refundBackups[id] = s.refund
	return int(id)
}

func (s *StateDB) AddLog(log *types.Log) {
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

func (s *StateDB) AddPreimage(common.Hash, []byte) {
	panic("not implemented")
}

func (s *StateDB) Witness() *stateless.Witness {
	return nil
}

func (s *StateDB) AccessEvents() *state.AccessEvents {
	panic("not implemented")
}

func (s *StateDB) Finalise(bool) {
	panic("not implemented")
}
