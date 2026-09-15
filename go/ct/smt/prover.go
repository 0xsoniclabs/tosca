// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

// Package smt proves properties of rule sets using the z3 SMT solver.
//
// Rules are encoded over the abstract state model declared by
// rlz.SmtStateModel. A rule set is complete if every state is matched by at
// least one rule, and sound if all rules matching a state have the same effect.
// Effects are compared by identity: all rules without effect are equivalent, so
// are all rules with a failing effect and all rules sharing a name; any other
// two effects are considered different.
package smt

import (
	"errors"
	"fmt"
	"os/exec"
	"regexp"
	"strconv"
	"strings"

	"github.com/0xsoniclabs/tosca/go/ct/rlz"
)

// CheckCompleteness returns the model of a state not matched by any rule, or
// an empty string if every state is matched by some rule.
func CheckCompleteness(rules []rlz.Rule) (string, error) {
	matches := make([]string, len(rules))
	for i := range rules {
		matches[i] = ruleName(i)
	}
	covered := "false"
	if len(matches) > 0 {
		covered = fmt.Sprintf("(or %s)", strings.Join(matches, " "))
	}
	problem := encode(rules) + fmt.Sprintf("(assert (not %s))\n(check-sat)\n", covered)
	return solve(problem)
}

// Conflict is a state matched by two rules with different effects.
type Conflict struct {
	First, Second rlz.Rule
	Model         string
}

// CheckSoundness returns up to limit pairs of rules with different effects that
// match a common state.
func CheckSoundness(rules []rlz.Rule, limit int) ([]Conflict, error) {
	var problem strings.Builder
	problem.WriteString(encode(rules))
	problem.WriteString("(declare-const i Int)\n(declare-const j Int)\n")
	problem.WriteString("(declare-const effect_i Int)\n(declare-const effect_j Int)\n")
	fmt.Fprintf(&problem, "(assert (and (<= 0 i) (< i j) (< j %d)))\n", len(rules))
	problem.WriteString("(assert (distinct effect_i effect_j))\n")
	for k, effect := range effectClasses(rules) {
		fmt.Fprintf(&problem, "(assert (=> (= i %d) (and %s (= effect_i %d))))\n", k, ruleName(k), effect)
		fmt.Fprintf(&problem, "(assert (=> (= j %d) (and %s (= effect_j %d))))\n", k, ruleName(k), effect)
	}

	conflicts := []Conflict{}
	for len(conflicts) < limit {
		model, err := solve(problem.String() + "(check-sat)\n")
		if err != nil || model == "" {
			return conflicts, err
		}
		i, j, err := selectedRules(model)
		if err != nil {
			return conflicts, err
		}
		conflicts = append(conflicts, Conflict{First: rules[i], Second: rules[j], Model: model})
		fmt.Fprintf(&problem, "(assert (not (and (= i %d) (= j %d))))\n", i, j)
	}
	return conflicts, nil
}

func encode(rules []rlz.Rule) string {
	var res strings.Builder
	res.WriteString("(set-option :dump-models true)\n")
	res.WriteString(rlz.SmtStateModel())
	for i, rule := range rules {
		fmt.Fprintf(&res, "; %s\n(define-fun %s () Bool %s)\n", rule.Name, ruleName(i), rule.Condition.ToSmt())
	}
	return res.String()
}

func ruleName(index int) string {
	return fmt.Sprintf("rule_%d", index)
}

func effectClasses(rules []rlz.Rule) []int {
	classes := map[any]int{}
	res := make([]int, len(rules))
	for i, rule := range rules {
		var key any = rule.Name
		switch rule.Effect {
		case rlz.NoEffect(), rlz.FailEffect():
			key = rule.Effect
		}
		class, found := classes[key]
		if !found {
			class = len(classes)
			classes[key] = class
		}
		res[i] = class
	}
	return res
}

var selectedRulePattern = regexp.MustCompile(`\(define-fun (i|j) \(\) Int\s+(\d+)\)`)

func selectedRules(model string) (int, int, error) {
	selected := map[string]int{}
	for _, match := range selectedRulePattern.FindAllStringSubmatch(model, -1) {
		selected[match[1]], _ = strconv.Atoi(match[2])
	}
	if len(selected) != 2 {
		return 0, 0, fmt.Errorf("model does not identify the conflicting rules: %s", model)
	}
	return selected["i"], selected["j"], nil
}

// solve runs z3 on the given problem and returns the model of the state if it
// is satisfiable or an empty string if it is not.
func solve(problem string) (string, error) {
	cmd := exec.Command("z3", "-in")
	cmd.Stdin = strings.NewReader(problem)
	output, err := cmd.Output()
	if err != nil {
		var exitErr *exec.ExitError
		if errors.As(err, &exitErr) {
			return "", fmt.Errorf("z3 failed: %w\n%s%s", err, output, exitErr.Stderr)
		}
		return "", fmt.Errorf("failed to run z3: %w", err)
	}
	verdict, model, _ := strings.Cut(string(output), "\n")
	switch verdict {
	case "sat":
		return stateModel(model), nil
	case "unsat":
		return "", nil
	}
	return "", fmt.Errorf("unexpected z3 output: %s", output)
}

var encodingSymbolPattern = regexp.MustCompile(`^\(define-fun (rule_\d+|effect_[ij]) `)

// stateModel drops the definitions of the rule encoding from a model, keeping
// the definitions of the state.
func stateModel(model string) string {
	var res strings.Builder
	depth, start := 0, 0
	for pos, char := range model {
		switch char {
		case '(':
			if depth == 1 {
				start = pos
			}
			depth++
		case ')':
			depth--
			if depth == 1 {
				if definition := model[start : pos+1]; !encodingSymbolPattern.MatchString(definition) {
					res.WriteString(definition + "\n")
				}
			}
		}
	}
	return res.String()
}
