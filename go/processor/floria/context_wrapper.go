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

// SelfDestruct overrides the SelfDestruct method of the embedded TransactionContext and
// performs the balance update according to the specified revision.
func (c floriaContext) SelfDestruct(address tosca.Address, beneficiary tosca.Address) bool {
	balance := c.GetBalance(address)
	if c.revision >= tosca.R13_Cancun {
		// Starting with Cancun, eip-6780 changes the behavior of selfdestruct.
		// The account is only deleted if selfdestruct is called within the same transaction
		// it has been created. The balance is transferred to the beneficiary, therefore it
		// has to be set to zero for the address being selfdestructed.
		c.SetBalance(address, tosca.Value{})
	}
	c.SetBalance(beneficiary, tosca.Add(c.GetBalance(beneficiary), balance))
	return c.TransactionContext.SelfDestruct(address, beneficiary)
}
