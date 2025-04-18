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
	"testing"

	"github.com/ethereum/go-ethereum/core/vm"
)

func TestStateDB_implementsVmStateDBInterface(t *testing.T) {
	var _ vm.StateDB = &StateDB{}
}
