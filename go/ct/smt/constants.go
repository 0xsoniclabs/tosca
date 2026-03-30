// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package smt

// VM constants matching spec_prover.py
const (
	NumStatusCodes   = 4
	NumRevisions     = 7
	MaxCodeSize      = 16384 + 8192       // 24576 — maximum deployed contract code size
	MaxCodeLen       = 2 * (16384 + 8192) // 49152 — maximum init code size, used in isCode/isData bounds
	MaxStackSize     = 1024
	NumStorageStatus = 9

	// Storage configuration values
	StorageAssigned         = 0
	StorageAdded            = 1
	StorageDeleted          = 2
	StorageModified         = 3
	StorageDeletedAdded     = 4
	StorageModifiedDeleted  = 5
	StorageDeletedRestored  = 6
	StorageAddedDeleted     = 7
	StorageModifiedRestored = 8
)
