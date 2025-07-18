// Copyright (c) 2025 Sonic Operations Ltd
//
// Use of this software is governed by the Business Source License included
// in the LICENSE file and at soniclabs.com/bsl11.
//
// Change Date: 2028-4-16
//
// On the date above, in accordance with the Business Source License, use of
// this software will be governed by the GNU Lesser General Public License v3.

pipeline {
    agent { label 'pr' }

    options {
        timestamps()
        timeout(time: 2, unit: 'HOURS')
        disableConcurrentBuilds(abortPrevious: true)
    }

    environment {
        CC = 'gcc'
        CXX = 'g++'
        PATH = "${env.HOME}/.cargo/bin:${env.PATH}"
    }

    stages {
        stage('Validate commit') {
            steps {
                script {
                    def CHANGE_REPO = sh(script: 'basename -s .git `git config --get remote.origin.url`', returnStdout: true).trim()
                    build job: '/Utils/Validate-Git-Commit', parameters: [
                        string(name: 'Repo', value: "${CHANGE_REPO}"),
                        string(name: 'Branch', value: "${env.CHANGE_BRANCH}"),
                        string(name: 'Commit', value: "${GIT_COMMIT}")
                    ]
                }
            }
        }

        stage('Checkout code') {
            steps {
                sh 'git submodule update --init --recursive'
            }
        }

        stage('Check License headers') {
            steps {
                sh 'cd scripts/license && ./add_license_header.sh --check'
            }
        }

        stage('Check Go formatting and lint Go sources') {
            steps {
                    sh 'make lint-go'
            }
        }

        stage('Check C++ sources formatting') {
            steps {
                sh 'find cpp/ -not -path "cpp/build/*" \\( -iname *.h -o -iname *.cc \\) | xargs clang-format --dry-run -Werror'
            }
        }

        stage('Build Go') {
            steps {
                sh 'make tosca-go'
            }
        }

        stage('Run Go tests') {
            environment {
                CODECOV_TOKEN = credentials('codecov-uploader-0xsoniclabs-global')
            }
            steps {
                sh 'make test-go'
                sh ('codecov upload-process -r 0xsoniclabs/tosca -f ./cover.out -t ${CODECOV_TOKEN}')
            }
        }

        stage('CT regression tests LFVM') {
            steps {
                sh 'go run ./go/ct/driver regressions lfvm'
            }
        }

        stage('CT regression tests evmzero') {
            steps {
                sh 'go run ./go/ct/driver regressions evmzero'
            }
        }

        stage('LFVM race condition tests') {
            steps {
                sh 'GORACE="halt_on_error=1" go test --race ./go/interpreter/lfvm/...'
            }
        }

        stage('Run C++ tests') {
            steps {
                sh 'make test-cpp'
            }
        }

        stage('Test C++ coverage support') {
            steps {
                sh 'make tosca-cpp-coverage'
                sh 'go test -v  -run ^TestDumpCppCoverageData$ ./go/lib/cpp/ --expect-coverage'
            }
        }

        stage('Test Rust coverage support') {
            steps {
                sh 'make tosca-rust-coverage'
                sh 'go test -v  -run ^TestDumpRustCoverageData$ ./go/lib/rust/ --expect-coverage'
            }
        }
    }
}
