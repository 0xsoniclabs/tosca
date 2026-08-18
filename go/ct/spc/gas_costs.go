// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package spc

import (
	"strings"

	. "github.com/0xsoniclabs/tosca/go/ct/common"
	"github.com/0xsoniclabs/tosca/go/tosca"
)

// This file collects the prices of the operations touching durable state, all
// of which changed with Amsterdam.
//
// EIP-2929 (Berlin) split each such access into an expensive first (cold) and a
// cheap repeated (warm) touch. EIP-8038 (Amsterdam) reprices those accesses and
// splits every write into an access and a write component. On top of that,
// EIP-8037 (Amsterdam) charges the durable growth of the state in a second gas
// dimension, fed from a reservoir provided alongside the regular gas.
//
// The CT models a single gas dimension: it never funds a state-gas reservoir,
// which leaves every state-dimension charge to fall back to regular gas (see
// tosca.Gas). The prices below are therefore the total an operation takes from
// the gas of the executing frame, with the state-dimension share named
// separately wherever an operation can hand it back.

const (
	// The prices of EIP-2929, in effect from Berlin up to Osaka.
	coldAccountAccessBerlin = 2600
	coldStorageAccessBerlin = 2100
	warmAccess              = 100

	// The gas a callee is granted on top of its budget when a call attaches a
	// non-zero value, unchanged since EIP-150.
	callStipend = 2300

	// The prices of EIP-8038, mirroring the params.*Amsterdam constants of
	// go-ethereum.
	coldAccountAccessAmsterdam  = 3000
	coldStorageAccessAmsterdam  = 3000
	accountWriteAmsterdam       = 8000
	storageWriteAmsterdam       = 10000
	storageClearRefundAmsterdam = 12480
	createAccessAmsterdam       = 11000

	// The state dimension of EIP-8037: durable state growth is priced per byte
	// of the state entry it creates.
	costPerStateByteAmsterdam = 1530
	accountCreationAmsterdam  = 120 * costPerStateByteAmsterdam
	storageCreationAmsterdam  = 64 * costPerStateByteAmsterdam
)

// revisionRange is a range of revisions, both ends included, sharing one set of
// state-access prices.
type revisionRange struct {
	first, last tosca.Revision
}

// name is a suffix identifying the range in the name of a rule.
func (r revisionRange) name() string {
	return strings.ToLower(r.first.String())
}

// repricedRevisionRanges splits the revisions from the given one onwards into
// the ranges sharing one set of state-access prices. A rule whose gas cost is
// one of the prices below needs one instance per range.
func repricedRevisionRanges(first tosca.Revision) []revisionRange {
	return []revisionRange{
		{first, tosca.R15_Osaka},
		{tosca.R16_Amsterdam, NewestSupportedRevision},
	}
}

// coldAccountAccessCost is the price of the first touch of an account within a
// transaction. A repeated touch costs warmAccess in every revision.
func coldAccountAccessCost(revision tosca.Revision) tosca.Gas {
	if revision >= tosca.R16_Amsterdam {
		return coldAccountAccessAmsterdam
	}
	return coldAccountAccessBerlin
}

// coldStorageAccessCost is the price of the first touch of a storage slot
// within a transaction. A repeated touch costs warmAccess in every revision.
func coldStorageAccessCost(revision tosca.Revision) tosca.Gas {
	if revision >= tosca.R16_Amsterdam {
		return coldStorageAccessAmsterdam
	}
	return coldStorageAccessBerlin
}

// codeReadCost is what EXTCODESIZE and EXTCODECOPY pay on top of the access to
// the account for reading its code, which EIP-8038 accounts for as a second
// database lookup.
func codeReadCost(revision tosca.Revision) tosca.Gas {
	if revision >= tosca.R16_Amsterdam {
		return warmAccess
	}
	return 0
}

// callValueTransferCost is the price of attaching a non-zero value to a call.
// It contains the stipend granted to the callee.
func callValueTransferCost(revision tosca.Revision) tosca.Gas {
	if revision >= tosca.R16_Amsterdam {
		return accountWriteAmsterdam + callStipend
	}
	return 9000
}

// accountCreationStateCost is the share of creating an account charged in the
// state dimension, priced by the size of the account entry it adds to the state.
// An operation that ends up not creating the account after all hands it back.
func accountCreationStateCost(revision tosca.Revision) tosca.Gas {
	if revision >= tosca.R16_Amsterdam {
		return accountCreationAmsterdam
	}
	return 0
}

// callAccountCreationCost is what a call attaching a value pays for the empty
// account it funds. Since Amsterdam the write to the account is part of
// callValueTransferCost, leaving only the state dimension of the creation.
func callAccountCreationCost(revision tosca.Revision) tosca.Gas {
	if revision >= tosca.R16_Amsterdam {
		return accountCreationStateCost(revision)
	}
	return 25000
}

// selfDestructAccountCreationCost is what SELFDESTRUCT pays for the empty
// account it sends the balance of the destructed contract to. Unlike a call it
// has no value transfer to fold the write to the account into, so since
// Amsterdam it pays for that write next to the state dimension of the creation.
func selfDestructAccountCreationCost(revision tosca.Revision) tosca.Gas {
	if revision >= tosca.R16_Amsterdam {
		return accountWriteAmsterdam + accountCreationStateCost(revision)
	}
	return 25000
}

// createAccessCost is the constant price of the CREATE and CREATE2 operations.
func createAccessCost(revision tosca.Revision) tosca.Gas {
	if revision >= tosca.R16_Amsterdam {
		return createAccessAmsterdam
	}
	return 32000
}
