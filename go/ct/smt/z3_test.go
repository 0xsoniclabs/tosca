package smt

import (
	_ "embed"
	"testing"

	"github.com/stretchr/testify/require"
)

func TestEval_CanDetermineSatisfiable(t *testing.T) {
	require := require.New(t)
	result, err := Eval(`
		(declare-const x Int) 
		(assert (and (< 5 x) (< x 7))) 
	`)
	require.NoError(err)
	require.True(result.Satisfiable)
	require.Equal("((define-fun x () Int 6))", result.Model)
}

func TestEval_CanDetermineUnsatisfiable(t *testing.T) {
	require := require.New(t)
	result, err := Eval(`
		(declare-const x Int) 
		(assert (and (> x 5) (< x 3)))
	`)
	require.NoError(err)
	require.False(result.Satisfiable)
	require.Empty(result.Model)
}

//go:embed completeness.smt2
var completenessProblem string

func TestEval_RunExampleCompletenessCheck(t *testing.T) {
	require := require.New(t)
	result, err := Eval(completenessProblem)
	require.NoError(err)
	require.False(result.Satisfiable)
	require.Empty(result.Model)
}
