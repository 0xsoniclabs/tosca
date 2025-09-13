package smt

import (
	"testing"

	"github.com/0xsoniclabs/tosca/go/ct/spc"
	"github.com/stretchr/testify/require"
)

// Development command:
//  clear && go test ./ct/smt -run TestCompleteness && z3 ./ct/smt/out.smt2

func TestCompleteness(t *testing.T) {
	require.NoError(t, CheckCompleteness(spc.Spec))
}
