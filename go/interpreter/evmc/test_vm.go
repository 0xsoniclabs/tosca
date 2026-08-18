// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package evmc

// This file provides an EVMC VM for the tests of this package. It cannot live in
// a test file, since Go does not support cgo in those.

/*
#include <evmc/evmc.h>
#include <string.h>

// testVmExecute returns the blob hashes the host reports as its output. Asking
// for the recipient makes the host consult its run context. The hashes stay
// pinned by the bindings until the output has been copied out of them.
static struct evmc_result testVmExecute(struct evmc_vm* vm,
                                        const struct evmc_host_interface* host,
                                        struct evmc_host_context* context,
                                        enum evmc_revision rev,
                                        const struct evmc_message* msg,
                                        uint8_t const* code, size_t code_size) {
	(void) vm; (void) rev; (void) code; (void) code_size;

	host->account_exists(context, &msg->recipient);
	struct evmc_tx_context tx = host->get_tx_context(context);

	struct evmc_result result;
	memset(&result, 0, sizeof(result));
	result.status_code = EVMC_SUCCESS;
	result.output_data = (const uint8_t*) tx.blob_hashes;
	result.output_size = tx.blob_hashes_count * sizeof(evmc_bytes32);
	return result;
}

static void testVmDestroy(struct evmc_vm* vm) { (void) vm; }

static enum evmc_capabilities testVmGetCapabilities(struct evmc_vm* vm) {
	(void) vm;
	return EVMC_CAPABILITY_EVM1;
}

static struct evmc_vm testVm = {
	EVMC_ABI_VERSION, "test", "0", testVmDestroy, testVmExecute, testVmGetCapabilities, NULL,
};

static struct evmc_vm* getTestVm() { return &testVm; }
*/
import "C"

import (
	"unsafe"

	"github.com/ethereum/evmc/v11/bindings/go/evmc"
)

// newTestInterpreter provides an interpreter backed by the VM above, which
// answers every execution from the transaction context its host reports.
func newTestInterpreter() *EvmcInterpreter {
	handle := unsafe.Pointer(C.getTestVm())
	return &EvmcInterpreter{vm: (*evmc.VM)(unsafe.Pointer(&handle))}
}
