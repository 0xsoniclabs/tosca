// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

package lfvm

import (
	"fmt"
	"io"
)

// loggingRunner is a runner that logs the execution of the contract code to an io.Writer.
// If the output log is nil, nothing is logged.
type loggingRunner struct {
	log io.Writer
}

// newLogger creates a new logging runner that writes to the provided
// io.Writer.
func newLogger(writer io.Writer) loggingRunner {
	return loggingRunner{log: writer}
}

func (l loggingRunner) run(c *context) (status, error) {
	status := statusRunning
	var err error
	for status == statusRunning {
		// log format: <op>, <gas>, <top-of-stack>\n
		if int(c.pc) < len(c.code) {
			top := "-empty-"
			if c.stack.len() > 0 {
				top = c.stack.peek().ToBig().String()
			}
			if l.log != nil {
				_, err = fmt.Fprintf(l.log, "%v, %d, %v\n", c.code[c.pc].opcode, c.gas, top)
				if err != nil {
					return status, err
				}
			}
		}
		status = execute(c, true)
	}
	return status, nil
}
