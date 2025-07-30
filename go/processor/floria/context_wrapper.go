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
	// the original selfdestruct function is saved here, as it still needs to be called
	selfdestruct func(addr, beneficiary tosca.Address) bool
}

func (c floriaContext) SelfDestruct(address tosca.Address, beneficiary tosca.Address) bool {
	balance := c.GetBalance(address)
	if c.revision >= tosca.R13_Cancun {
		c.SetBalance(address, tosca.Value{})
	}
	c.SetBalance(beneficiary, tosca.Add(c.GetBalance(beneficiary), balance))
	return c.selfdestruct(address, beneficiary)
}
