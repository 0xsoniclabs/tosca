// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package floria_eth

import (
	"testing"

	"github.com/0xsoniclabs/tosca/go/tosca"
	"go.uber.org/mock/gomock"
)

func TestProcessor_NewProcessorReturnsProcessor(t *testing.T) {
	interpreter := tosca.NewMockInterpreter(gomock.NewController(t))
	processor := newFloriaEthProcessor(interpreter)
	if processor == nil {
		t.Errorf("newProcessor returned nil")
	}
}

func TestProcessorRegistry_InitProcessor(t *testing.T) {
	processorFactories := tosca.GetAllRegisteredProcessorFactories()
	if len(processorFactories) == 0 {
		t.Errorf("No processor factories found")
	}

	processor := tosca.GetProcessorFactory("floria-eth")
	if processor == nil {
		t.Errorf("Floria processor factory not found")
	}
}
