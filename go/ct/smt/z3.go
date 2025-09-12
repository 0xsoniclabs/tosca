package smt

import (
	"bufio"
	"fmt"
	"io"
	"os/exec"
	"regexp"
	"strings"
	"sync"

	"github.com/0xsoniclabs/tosca/go/ct/common"
)

const (
	ErrToolNotInstalled = common.ConstErr("z3: tool not installed")
)

type Result struct {
	Satisfiable bool
	Model       string // only valid if Satisfiable is true
}

func Eval(problem string) (Result, error) {
	var res Result

	cmd := exec.Command("z3", "-in", "-T:30")

	stdin, err := cmd.StdinPipe()
	if err != nil {
		return res, fmt.Errorf("failed to get stdin pipe: %v", err)
	}

	stdout, err := cmd.StdoutPipe()
	if err != nil {
		return res, fmt.Errorf("failed to get stdout pipe: %v", err)
	}

	stderr, err := cmd.StderrPipe()
	if err != nil {
		return res, fmt.Errorf("failed to get stderr pipe: %v", err)
	}

	satisfiable := make(chan bool, 1)

	wg := sync.WaitGroup{}
	wg.Add(3)

	go func() {
		defer wg.Done()
		defer stdout.Close()
		reader := bufio.NewReader(stdout)
		line, err := reader.ReadString('\n')
		if err != nil {
			panic(err)
		}
		if strings.Contains(line, "unsat") {
			satisfiable <- false
			io.Copy(io.Discard, reader)
			return
		}
		satisfiable <- true
		res.Satisfiable = true
		model, err := io.ReadAll(reader)
		if err != nil {
			panic(err)
		}
		res.Model = string(model)
	}()

	go func() {
		defer wg.Done()
		defer stderr.Close()
		out, err := io.ReadAll(stderr)
		fmt.Printf("Z3 stderr: %s (err: %v)\n", out, err)
	}()

	go func() {
		defer wg.Done()
		defer stdin.Close()
		io.WriteString(stdin, problem)
		io.WriteString(stdin, "\n(check-sat)\n")
		sat := <-satisfiable
		if sat {
			io.WriteString(stdin, "(get-model)\n")
		}
		io.WriteString(stdin, "(exit)\n")
	}()

	if err := cmd.Start(); err != nil {
		return res, fmt.Errorf("failed to start z3: %v", err)
	}

	if err := cmd.Wait(); err != nil {
		return res, fmt.Errorf("failed to wait for z3: %v", err)
	}
	wg.Wait()

	// Model output cleanup ...
	res.Model = strings.ReplaceAll(res.Model, "\n", "")

	re := regexp.MustCompile(`\s+`)
	res.Model = re.ReplaceAllString(res.Model, " ")

	re = regexp.MustCompile(`\(\s+\(`)
	res.Model = re.ReplaceAllString(res.Model, "((")

	re = regexp.MustCompile(`\)\s+\)`)
	res.Model = re.ReplaceAllString(res.Model, "))")

	return res, nil
}
