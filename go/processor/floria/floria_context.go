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

import "github.com/0xsoniclabs/tosca/go/tosca"

type floriaContext struct {
	context tosca.TransactionContext
}

func (c floriaContext) SelfDestruct(addr tosca.Address, beneficiary tosca.Address) bool {
	c.context.SetBalance(beneficiary, tosca.Add(c.context.GetBalance(beneficiary), c.context.GetBalance(addr)))
	return c.context.SelfDestruct(addr, beneficiary)
}

func (c floriaContext) GetBalance(address tosca.Address) tosca.Value {
	return c.context.GetBalance(address)
}

func (c floriaContext) SetBalance(address tosca.Address, value tosca.Value) {
	c.context.SetBalance(address, value)
}

func (c floriaContext) GetNonce(address tosca.Address) uint64 {
	return c.context.GetNonce(address)
}

func (c floriaContext) SetNonce(address tosca.Address, nonce uint64) {
	c.context.SetNonce(address, nonce)
}

func (c floriaContext) GetCode(address tosca.Address) tosca.Code {
	return c.context.GetCode(address)
}

func (c floriaContext) GetCodeHash(address tosca.Address) tosca.Hash {
	return c.context.GetCodeHash(address)
}

func (c floriaContext) GetCodeSize(address tosca.Address) int {
	return c.context.GetCodeSize(address)
}

func (c floriaContext) SetCode(address tosca.Address, code tosca.Code) {
	c.context.SetCode(address, code)
}

func (c floriaContext) GetStorage(address tosca.Address, key tosca.Key) tosca.Word {
	return c.context.GetStorage(address, key)
}

func (c floriaContext) SetStorage(address tosca.Address, key tosca.Key, word tosca.Word) tosca.StorageStatus {
	return c.context.SetStorage(address, key, word)
}

func (c floriaContext) CreateSnapshot() tosca.Snapshot {
	return c.context.CreateSnapshot()
}

func (c floriaContext) RestoreSnapshot(snapshot tosca.Snapshot) {
	c.context.RestoreSnapshot(snapshot)
}

func (c floriaContext) GetTransientStorage(address tosca.Address, key tosca.Key) tosca.Word {
	return c.context.GetTransientStorage(address, key)
}

func (c floriaContext) SetTransientStorage(address tosca.Address, key tosca.Key, word tosca.Word) {
	c.context.SetTransientStorage(address, key, word)
}

func (c floriaContext) AccessAccount(address tosca.Address) tosca.AccessStatus {
	return c.context.AccessAccount(address)
}

func (c floriaContext) AccessStorage(address tosca.Address, key tosca.Key) tosca.AccessStatus {
	return c.context.AccessStorage(address, key)
}

func (c floriaContext) EmitLog(log tosca.Log) {
	c.context.EmitLog(log)
}

func (c floriaContext) GetLogs() []tosca.Log {
	return c.context.GetLogs()
}

func (c floriaContext) GetBlockHash(number int64) tosca.Hash {
	return c.context.GetBlockHash(number)
}

func (c floriaContext) GetCommittedStorage(addr tosca.Address, key tosca.Key) tosca.Word {
	//lint:ignore SA1019 deprecated functions to be migrated
	return c.context.GetCommittedStorage(addr, key)
}

func (c floriaContext) IsAddressInAccessList(addr tosca.Address) bool {
	//lint:ignore SA1019 deprecated functions to be migrated
	return c.context.IsAddressInAccessList(addr)
}

func (c floriaContext) IsSlotInAccessList(addr tosca.Address, key tosca.Key) (addressPresent, slotPresent bool) {
	//lint:ignore SA1019 deprecated functions to be migrated
	return c.context.IsSlotInAccessList(addr, key)
}

func (c floriaContext) HasSelfDestructed(addr tosca.Address) bool {
	//lint:ignore SA1019 deprecated functions to be migrated
	return c.context.HasSelfDestructed(addr)
}

func (c floriaContext) AccountExists(address tosca.Address) bool {
	return c.context.AccountExists(address)
}
