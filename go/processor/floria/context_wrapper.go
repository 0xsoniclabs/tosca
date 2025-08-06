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

import "github.com/0xsoniclabs/tosca/go/tosca"

// floriaContext is a wrapper around the tosca.TransactionContext
// that adds the balance transfer to the selfdestruct function
type floriaContext struct {
	tosca.TransactionContext
	revision tosca.Revision
}

// SelfDestruct overrides the SelfDestruct method to perform the balance update
// based on the specified revision. Geth handles selfdestruct balance updates
// within the interpreter, but in Tosca, the updates are managed by the processor
// for consistency with calls and creates.
func (c floriaContext) SelfDestruct(address tosca.Address, beneficiary tosca.Address) bool {
	balance := c.GetBalance(address)
	if c.revision >= tosca.R13_Cancun {
		// Pre Cancun selfdestructed accounts were deleted, this is no longer the case since eip-6780.
		// To ensure no balance is left on the selfdestructed account, the balance is set to zero.
		c.SetBalance(address, tosca.Value{})
	}
	c.SetBalance(beneficiary, tosca.Add(c.GetBalance(beneficiary), balance))
	return c.TransactionContext.SelfDestruct(address, beneficiary)
}
