cmake_minimum_required(VERSION 3.21)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

project(tosca LANGUAGES C CXX) 

list(APPEND CMAKE_MODULE_PATH "${CMAKE_CURRENT_SOURCE_DIR}/cmake")

set(CMAKE_EXPORT_COMPILE_COMMANDS ON)
set(CMAKE_POSITION_INDEPENDENT_CODE TRUE)
enable_testing()

include(tosca_dependencies)
include(tosca_asan)
include(tosca_assert)
include(tosca_coverage)

set(TOSCA_THIRD_PARTY_DIR ${PROJECT_SOURCE_DIR}/../third_party)
add_subdirectory(${TOSCA_THIRD_PARTY_DIR}/evmc third_party/evmc EXCLUDE_FROM_ALL)

add_subdirectory(common)
add_subdirectory(vm/evmzero)
