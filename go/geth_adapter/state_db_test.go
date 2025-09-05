// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package geth_adapter

import (
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca"
	common "github.com/ethereum/go-ethereum/common"
	"github.com/ethereum/go-ethereum/core/vm"
	"github.com/stretchr/testify/require"
	"go.uber.org/mock/gomock"
)

func TestStateDB_implementsVmStateDBInterface(t *testing.T) {
	var _ vm.StateDB = &StateDB{}
}

func TestStateDB_GetStateAndCommittedStateReturnsOriginalAndCurrentState(t *testing.T) {
	ctrl := gomock.NewController(t)
	context := tosca.NewMockTransactionContext(ctrl)

	address := tosca.Address{0x1}
	key := tosca.Key{0x2}
	original := tosca.Word{0x3}
	current := tosca.Word{0x4}

	context.EXPECT().GetStorage(address, key).Return(original)
	context.EXPECT().GetCommittedStorage(address, key).Return(current)
	stateDB := NewStateDB(context)

	state, committed := stateDB.GetStateAndCommittedState(common.Address(address), common.Hash(key))
	require.Equal(t, original, tosca.Word(state))
	require.Equal(t, current, tosca.Word(committed))
}
