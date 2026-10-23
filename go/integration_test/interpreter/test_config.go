// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package interpreter_test

import (
	"slices"
	"strings"

	_ "github.com/0xsoniclabs/tosca/go/interpreter/evmrs"
	_ "github.com/0xsoniclabs/tosca/go/interpreter/geth"
	_ "github.com/0xsoniclabs/tosca/go/interpreter/sfvm"
	"github.com/0xsoniclabs/tosca/go/tosca"
	"golang.org/x/exp/maps"
)

// getAllInterpreterVariantsForTests returns all registered interpreter variants
// that should be covered in integration tests.
func getAllInterpreterVariantsForTests() []string {
	// TODO: re-add logging variants once logging is no longer writing everything to stdout
	return slices.DeleteFunc(
		maps.Keys(tosca.GetAllRegisteredInterpreters()),
		func(s string) bool { return strings.Contains(s, "logging") },
	)
}
