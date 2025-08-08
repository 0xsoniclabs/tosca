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

import (
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"go.uber.org/mock/gomock"
)

func TestFloriaContext_SelfDestructPerformsTheBalanceUpdate(t *testing.T) {
	revisions := tosca.GetAllKnownRevisions()
	for _, revision := range revisions {
		t.Run(revision.String(), func(t *testing.T) {
			ctrl := gomock.NewController(t)
			context := tosca.NewMockTransactionContext(ctrl)

			beneficiary := tosca.Address{0x01}
			address := tosca.Address{0x02}
			balance := tosca.NewValue(1000)
			beneficiaryBalance := tosca.NewValue(10)

			context.EXPECT().GetBalance(address).Return(balance)
			context.EXPECT().SetBalance(address, tosca.Value{})
			context.EXPECT().GetBalance(beneficiary).Return(beneficiaryBalance)
			context.EXPECT().SetBalance(beneficiary, tosca.Add(balance, beneficiaryBalance))
			context.EXPECT().SelfDestruct(address, beneficiary).Return(true)

			floriaContext := floriaContext{
				TransactionContext: context,
			}

			selfdestructed := floriaContext.SelfDestruct(address, beneficiary)
			if !selfdestructed {
				t.Errorf("SelfDestruct should return true, got false")
			}
		})
	}
}
