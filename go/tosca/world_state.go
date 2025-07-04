// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package tosca

import "fmt"

//go:generate mockgen -source world_state.go -destination world_state_mock.go -package tosca

// WorldState is an interface to access and manipulate the state of the block chain.
// The state of the chain is a collection of accounts, each with a balance, a nonce,
// optional code and storage.
type WorldState interface {
	AccountExists(Address) bool

	CreateAccount(Address)

	GetBalance(Address) Value
	SetBalance(Address, Value)

	GetNonce(Address) uint64
	SetNonce(Address, uint64)

	GetCode(Address) Code
	GetCodeHash(Address) Hash
	GetCodeSize(Address) int
	SetCode(Address, Code)

	// HasEmptyStorage returns whether the account has an empty storage.
	HasEmptyStorage(Address) bool
	GetStorage(Address, Key) Word
	SetStorage(Address, Key, Word) StorageStatus

	// SelfDestruct destroys addr and transfers its balance to beneficiary.
	// Returns true if the given account is destructed for the first time in the ongoing transaction, false otherwise.
	SelfDestruct(addr Address, beneficiary Address) bool
}

// Address represents the 160-bit (20 bytes) address of an account.
type Address [20]byte

// Key represents the 256-bit (32 bytes) key of a storage slot.
type Key [32]byte

// Word represents an arbitrary 256-bit (32 byte) word in the EVM.
type Word [32]byte

// Value represents an amount of chain currency, typically wei.
type Value [32]byte

// Hash represents the 256-bit (32 bytes) hash of a code, a block, a topic
// or similar sequence of cryptographic summary information.
type Hash [32]byte

// Code represents the byte-code of a contract.
type Code []byte

// StorageStatus is an enum utilized to indicate the effect of a storage
// slot update on the respective slot in the context of the current
// transaction. It is needed to perform proper gas price calculations of
// SSTORE operations.
type StorageStatus int

// See t.ly/b5HPf for the definition of these values.
const (
	// The comment indicates the storage values for the corresponding
	// configuration. X, Y, Z are non-zero numbers, distinct from each other,
	// while 0 is zero.
	//
	// <original> -> <current> -> <new>
	StorageAssigned         StorageStatus = iota
	StorageAdded                          // 0 -> 0 -> Z
	StorageDeleted                        // X -> X -> 0
	StorageModified                       // X -> X -> Z
	StorageDeletedAdded                   // X -> 0 -> Z
	StorageModifiedDeleted                // X -> Y -> 0
	StorageDeletedRestored                // X -> 0 -> X
	StorageAddedDeleted                   // 0 -> Y -> 0
	StorageModifiedRestored               // X -> Y -> X
)

func (config StorageStatus) String() string {
	switch config {
	case StorageAssigned:
		return "StorageAssigned"
	case StorageAdded:
		return "StorageAdded"
	case StorageAddedDeleted:
		return "StorageAddedDeleted"
	case StorageDeletedRestored:
		return "StorageDeletedRestored"
	case StorageDeletedAdded:
		return "StorageDeletedAdded"
	case StorageDeleted:
		return "StorageDeleted"
	case StorageModified:
		return "StorageModified"
	case StorageModifiedDeleted:
		return "StorageModifiedDeleted"
	case StorageModifiedRestored:
		return "StorageModifiedRestored"
	}
	return fmt.Sprintf("StorageStatus(%d)", config)
}

func GetAllStorageStatuses() []StorageStatus {
	return []StorageStatus{
		StorageAssigned,
		StorageAdded,
		StorageAddedDeleted,
		StorageDeletedRestored,
		StorageDeletedAdded,
		StorageDeleted,
		StorageModified,
		StorageModifiedDeleted,
		StorageModifiedRestored,
	}
}
