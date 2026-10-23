# Code Coverage

The Tosca project collects coverage data for the Go code base.

## Coverage in Go

The official [documentation](https://go.dev/doc/build-cover) explains how to use it.
Both Unit tests and CT driver can produce coverage reports.

By default Go coverage will only include the source of included files, not dependencies. 
The argument `-coverpkg=./go/...,github.com/project/dependency-name/` will include coverage reports from included code.
